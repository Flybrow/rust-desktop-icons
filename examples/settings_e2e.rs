mod support;

use serde_json::{Value, json};
use std::{env, fs, path::PathBuf, thread, time::Duration};
use support::{app, chord, click, fences, menu, poll, quit, rect, saved, type_text, wait, windows, work};
use windows::Win32::{
    Foundation::HWND,
    UI::{HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::*},
};

const TOP: i32 = 56;
const SIDE: i32 = 190;
const HEADER: i32 = 46;
const NOTE: i32 = 92;
const ROW: i32 = 44;
const DESCRIBED: i32 = 66;
const RESET: i32 = 22;
const SETTINGS: [VIRTUAL_KEY; 4] = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_F10];
const HIDE: [VIRTUAL_KEY; 4] = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_F9];
const NEW: [VIRTUAL_KEY; 4] = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_F8];

struct Ui {
    h: HWND,
    pid: u32,
    data: PathBuf,
    fails: Vec<String>,
}

impl Ui {
    fn check(&mut self, ok: bool, what: &str) {
        support::verdict(ok, what, &mut self.fails);
    }

    fn s(&self, v: i32) -> i32 {
        v * unsafe { GetDpiForWindow(self.h) } as i32 / 96
    }

    fn page(&self, i: i32) {
        let r = rect(self.h);
        click((r.left + self.s(60), r.top + self.s(TOP + 12 + 40 * i + 20)));
        thread::sleep(Duration::from_millis(300));
    }

    fn row_at(&self, heights: &[i32], i: usize, dy: i32) {
        let r = rect(self.h);
        let y = TOP + heights[..i].iter().sum::<i32>() + dy;
        click((r.left + self.s(SIDE + 150), r.top + self.s(y)));
        thread::sleep(Duration::from_millis(400));
    }

    fn row(&self, heights: &[i32], i: usize) {
        self.row_at(heights, i, 20);
    }

    fn reset(&self, heights: &[i32], i: usize) {
        self.row_at(heights, i, heights[i] - RESET / 2);
    }

    fn cfg(&self) -> Value {
        saved(&self.data)
    }

    fn until(&self, f: impl Fn(&Value) -> bool) -> bool {
        wait(3, || f(&saved(&self.data)))
    }

    fn pick(&self, path: &[&str]) -> bool {
        menu::open() && menu::pick(path)
    }
}

fn prompt(text: &str) {
    thread::sleep(Duration::from_millis(400));
    chord(&[VK_CONTROL, VK_A]);
    type_text(text);
    chord(&[VK_RETURN]);
    thread::sleep(Duration::from_millis(400));
}

fn main() {
    let w = work();
    let root = env::temp_dir().join(format!("rdi-settings-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    for id in [1, 2] {
        fs::create_dir_all(data.join("fences").join(id.to_string())).unwrap();
    }
    let fence = |id: u64, x: i32, title: &str| json!({"id": id, "x": w.left + x, "y": w.top + 700, "w": 300, "h": 200, "tabs": [{"id": id, "title": title}]});
    let mut docs = fence(1, 20, "Docs");
    docs["spacing"] = json!("Compact");
    docs["custom"] = json!(["Spacing"]);
    let cfg = json!({"lang": "en", "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"),
        "settings_key": {"mods": 7, "vk": 121}, "fences": [docs, fence(2, 340, "Archive")]});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let export = root.join("export.json");
    let mut child = app(&root, &[("RDI_PICK", export.to_str().unwrap())], &[]);
    let pid = child.id();
    let ok = wait(10, || fences(pid).len() == 2);
    thread::sleep(Duration::from_millis(800));
    chord(&SETTINGS);
    let Some(h) = poll(3, || windows(pid, "RustDesktopIcons.Settings").first().copied()) else {
        println!("[FAIL] the settings window opens");
        quit(&mut child);
        std::process::exit(1);
    };
    let mut u = Ui { h, pid, data: data.clone(), fails: vec![] };
    u.check(ok, "two fences shown");
    let f = |c: &Value, i: usize, k: &str| c["fences"][i][k].clone();

    let icons = [HEADER, NOTE, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED];
    u.page(1);
    u.row(&icons, 4);
    u.check(u.until(|c| c["look"]["thumbs"] == false && f(c, 1, "thumbs") == false), "Icons → Show thumbnails can be turned off");
    u.row(&icons, 4);
    u.until(|c| c["look"]["thumbs"] == true);
    u.row(&icons, 7);
    u.pick(&["Wide"]);
    u.check(
        u.until(|c| c["look"]["spacing"] == "Wide" && f(c, 1, "spacing") == "Wide" && f(c, 0, "spacing") == "Compact"),
        "Icons → Icon spacing applies to every fence except one that changed it",
    );
    u.row(&icons, 5);
    u.pick(&["Large"]);
    u.check(u.until(|c| c["look"]["label_size"] == "Large" && f(c, 0, "label_size") == "Large"), "Icons → Name size applies to the fences");
    u.row(&icons, 3);
    u.check(u.until(|c| c["look"]["labels"] == false && f(c, 1, "labels") == false), "Icons → Show names can be turned off");
    u.row(&icons, 3);
    u.until(|c| c["look"]["labels"] == true);
    u.row(&icons, 6);
    u.pick(&["Color"]);
    let dialog = support::answer(pid, &[VK_ESCAPE]);
    u.check(dialog && u.cfg()["look"]["label_color"].is_null(), "Icons → Name color opens the color picker; cancel keeps it automatic");
    u.row(&icons, 8);
    u.pick(&["Center"]);
    u.check(u.until(|c| c["look"]["align"] == "Center" && f(c, 0, "align") == "Center"), "Icons → Icon alignment applies to the fences");
    u.row(&icons, 2);
    u.pick(&["64 px"]);
    u.check(u.until(|c| c["look"]["icon"] == 64 && f(c, 1, "icon") == 64), "Icons → Icon size applies to the fences");

    let look = [HEADER, NOTE, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED];
    u.page(0);
    u.row(&look, 3);
    u.check(
        u.until(|c| c["look"]["wallpaper"] == true && f(c, 1, "wallpaper") == true),
        "Default appearance → Color from the wallpaper turns on",
    );
    u.row(&look, 3);
    u.until(|c| c["look"]["wallpaper"] == false);
    u.row(&look, 4);
    u.pick(&["70%"]);
    u.check(u.until(|c| c["look"]["alpha"].as_u64().is_some_and(|a| a.abs_diff(178) <= 2)), "Default appearance → Opacity is saved");
    u.row(&look, 5);
    u.pick(&["None"]);
    u.check(u.until(|c| c["look"]["tint"].is_null()), "Default appearance → Icon tint None");
    u.row(&look, 6);
    let asked = support::answer(pid, &[VK_RETURN]);
    u.check(
        asked && u.until(|c| f(c, 0, "custom") == json!([]) && f(c, 0, "spacing") == "Wide"),
        "Reset all fences asks, then every fence follows the settings",
    );

    let bar = [HEADER, NOTE, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED];
    u.page(2);
    u.row(&bar, 2);
    u.pick(&["Left"]);
    u.check(
        u.until(|c| c["look"]["title_align"] == "Left" && f(c, 1, "title_align") == "Left"),
        "Title bar → alignment applies to the fences",
    );
    u.row(&bar, 3);
    u.pick(&["Large"]);
    u.check(u.until(|c| c["look"]["title_size"] == "Large"), "Title bar → title size is saved");
    u.row(&bar, 4);
    u.pick(&["Automatic"]);
    u.check(u.until(|c| c["look"]["bar_color"].is_null()), "Title bar → bar color stays automatic");
    u.row(&bar, 5);
    u.check(u.until(|c| c["look"]["bar_hover"] == true && f(c, 1, "bar_hover") == true), "Title bar → show on hover turns on");
    u.row(&bar, 5);
    u.until(|c| c["look"]["bar_hover"] == false);
    chord(&[VK_CONTROL, VK_Z]);
    u.check(u.until(|c| c["look"]["bar_hover"] == true), "Ctrl+Z in the settings undoes the last change");

    let behave = [HEADER, NOTE, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, NOTE];
    u.page(3);
    for (i, key) in [(2, "chameleon"), (3, "auto_height"), (4, "browse")] {
        let before = u.cfg()["look"][key].as_bool().unwrap_or(false);
        u.row(&behave, i);
        let changed = u.until(|c| c["look"][key] == !before && f(c, 1, key) == !before);
        u.row(&behave, i);
        u.check(changed && u.until(|c| c["look"][key] == before), &format!("Behavior → {key} toggles for every fence"));
    }
    u.row(&behave, 5);
    u.pick(&["Instant"]);
    u.check(u.until(|c| c["roll_ms"] == 0), "Behavior → Unroll speed is saved");

    let desk = [HEADER, DESCRIBED, DESCRIBED, DESCRIBED];
    u.page(4);
    u.row(&desk, 3);
    let on = u.until(|c| c["dblclick_hide"] == true);
    u.row(&desk, 3);
    u.check(on && u.until(|c| c["dblclick_hide"] == false), "Desktop → Hide all on double-click toggles");
    u.row(&desk, 2);
    u.pick(&["Don't adjust"]);
    u.check(u.until(|c| c["screen_adapt"] == "Keep"), "Desktop → When the screen changes is saved");

    let mut keys = vec![HEADER, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED + RESET];
    u.page(6);
    u.row(&keys, 3);
    chord(&HIDE);
    u.check(u.until(|c| c["hide_key"] == json!({"mods": 7, "vk": 120})), "Shortcuts → a pressed combination becomes the hide shortcut");
    keys[3] += RESET;
    u.row(&keys, 4);
    chord(&SETTINGS);
    thread::sleep(Duration::from_millis(300));
    chord(&[VK_ESCAPE]);
    u.check(u.cfg()["new_key"].is_null(), "a combination already used by another shortcut is refused");
    u.row(&keys, 4);
    chord(&NEW);
    u.check(u.until(|c| c["new_key"] == json!({"mods": 7, "vk": 119})), "Shortcuts → new fence shortcut is saved");
    keys[4] += RESET;
    chord(&HIDE);
    let hidden = wait(3, || fences(pid).is_empty());
    chord(&HIDE);
    u.check(hidden && wait(3, || fences(pid).len() == 2), "the hide shortcut hides the fences, then brings them back");
    chord(&NEW);
    u.check(wait(3, || fences(pid).len() == 3), "the new fence shortcut creates a fence");
    thread::sleep(Duration::from_millis(400));
    let r = rect(u.h);
    let over = unsafe {
        windows::Win32::UI::WindowsAndMessaging::WindowFromPoint(windows::Win32::Foundation::POINT {
            x: (r.left + r.right) / 2,
            y: (r.top + r.bottom) / 2,
        })
    };
    let top = unsafe { windows::Win32::UI::WindowsAndMessaging::GetAncestor(over, windows::Win32::UI::WindowsAndMessaging::GA_ROOT) };
    u.check(top == u.h, "a new fence stays behind the open windows");
    u.reset(&keys, 3);
    u.check(u.until(|c| c["hide_key"].is_null()), "Reset to default clears a changed shortcut");

    let n_tabs = u.cfg()["fences"].as_array().map_or(0, |a| a.len());
    let mut rules = vec![HEADER, DESCRIBED];
    rules.extend(std::iter::repeat_n(DESCRIBED, n_tabs));
    rules.extend([DESCRIBED, HEADER, DESCRIBED, HEADER, DESCRIBED]);
    let (add_name, add_age) = (n_tabs + 4, n_tabs + 6);
    u.page(5);
    u.row(&rules, 1);
    u.check(u.until(|c| c["auto_sort"] == true), "Sorting rules → automatic sorting toggles");
    u.row(&rules, 1);
    u.until(|c| c["auto_sort"] == false);
    u.row(&rules, 2);
    u.pick(&["Images"]);
    u.check(u.until(|c| f(c, 0, "tabs")[0]["kinds"] == json!(["Images"])), "Sorting rules → a fence row picks the file types it takes");
    u.row(&rules, add_name);
    prompt("*.e2e");
    u.pick(&["Archive"]);
    u.check(u.until(|c| c["name_rules"] == json!([{"pattern": "*.e2e", "fence": 2, "tab": 2}])), "Add a rule by name: pattern, then fence");
    let mut rules_named = rules.clone();
    rules_named.insert(add_name, ROW);
    u.row(&rules_named, add_name);
    u.pick(&["Edit the name pattern"]);
    prompt("*.log");
    u.check(u.until(|c| c["name_rules"][0]["pattern"] == "*.log"), "a name rule's pattern can be edited");
    u.row(&rules_named, add_name);
    u.pick(&["Change the destination fence", "Docs"]);
    u.check(u.until(|c| c["name_rules"][0]["fence"] == 1), "a name rule's fence can be changed");
    u.row(&rules_named, add_name);
    u.pick(&["Delete rule"]);
    u.check(u.until(|c| c["name_rules"].as_array().is_none_or(|a| a.is_empty())), "a name rule can be deleted");
    u.row(&rules, add_age);
    u.pick(&["Docs"]);
    u.pick(&["Older than 60 days"]);
    u.pick(&["Archive"]);
    let age = json!({"days": 60, "fence": 2, "tab": 2});
    u.check(u.until(|c| f(c, 0, "tabs")[0]["expire"] == age), "Add a rule by age: fence, age, destination");
    let mut rules_aged = rules.clone();
    rules_aged.insert(add_age, ROW);
    u.row(&rules_aged, add_age);
    u.pick(&["Older than 60 days"]);
    u.pick(&["Older than 90 days"]);
    u.check(u.until(|c| f(c, 0, "tabs")[0]["expire"]["days"] == 90), "an age rule's age can be changed");
    u.row(&rules_aged, add_age);
    u.pick(&["Delete rule"]);
    u.check(u.until(|c| f(c, 0, "tabs")[0]["expire"].is_null()), "an age rule can be deleted");

    let general = [HEADER, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED];
    u.page(7);
    u.row(&general, 1);
    u.pick(&["Français"]);
    let french = u.until(|c| c["lang"] == "fr");
    u.row(&general, 1);
    u.pick(&["English"]);
    u.check(french && u.until(|c| c["lang"] == "en"), "General → Language switches and back");
    u.row(&general, 3);
    u.check(u.until(|c| c["auto_update"] == true), "General → Automatic updates toggles");
    u.row(&general, 3);
    u.row(&general, 2);
    let off = u.until(|c| c["autostart"] == false);
    u.row(&general, 2);
    u.check(off && u.until(|c| c["autostart"] == true), "General → Start with Windows toggles");
    u.row(&general, 5);
    let exported: Value = poll(3, || fs::read(&export).ok().and_then(|b| serde_json::from_slice(&b).ok())).unwrap_or_default();
    u.check(
        exported["fences"].as_array().is_some_and(|a| a.len() == 3) && exported["look"]["spacing"] == "Wide",
        "Export writes the whole configuration",
    );
    let mut edited = exported.clone();
    edited["fences"].as_array_mut().unwrap().retain(|f| f["id"] != 2);
    edited["dblclick_hide"] = json!(true);
    edited["auto_update"] = json!(true);
    fs::write(&export, serde_json::to_vec(&edited).unwrap()).unwrap();
    let auto_update = u.cfg()["auto_update"].clone();
    u.row(&general, 6);
    let asked = support::answer(pid, &[VK_RETURN]);
    u.check(
        asked && u.until(|c| c["fences"].as_array().is_some_and(|a| a.len() == 2) && c["dblclick_hide"] == true),
        "Import asks, then replaces fences and settings",
    );
    u.check(u.cfg()["auto_update"] == auto_update, "Import keeps this PC's own settings");
    u.check(wait(3, || fences(pid).len() == 2), "the imported fences are shown");
    fs::write(&export, b"{ not json").unwrap();
    u.row(&general, 6);
    let warned = support::answer(pid, &[VK_RETURN]);
    u.check(warned && u.cfg()["fences"].as_array().is_some_and(|a| a.len() == 2), "a broken file is reported and changes nothing");

    chord(&[VK_ESCAPE]);
    u.check(wait(3, || windows(u.pid, "RustDesktopIcons.Settings").is_empty()), "Esc closes the settings");
    quit(&mut child);
    let _ = fs::remove_dir_all(&root);
    if u.fails.is_empty() {
        println!("SETTINGS E2E OK");
    } else {
        println!("{} check(s) failed", u.fails.len());
        std::process::exit(1);
    }
}
