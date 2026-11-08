mod support;

use serde_json::{Value, json};
use std::{env, fs, path::Path, thread, time::Duration};
use support::{
    app, chord, click, clip, close, ctrl_wheel, double_click, drag, explorer, fence_cfg, menu, poll, press, quit, rect, right_click, saved,
    type_text, wait, windows, work,
};
use windows::Win32::{
    Foundation::HWND,
    UI::{HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::*},
};

const PEEK: [VIRTUAL_KEY; 4] = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_F11];

struct Run {
    pid: u32,
    data: std::path::PathBuf,
    fails: Vec<String>,
}

impl Run {
    fn check(&mut self, ok: bool, what: &str) {
        support::verdict(ok, what, &mut self.fails);
    }

    fn hwnd(&self, title: &str) -> Option<HWND> {
        let id = saved(&self.data)["fences"].as_array()?.iter().find(|f| f["tabs"][0]["title"] == title)?["id"].as_u64()?;
        windows(self.pid, "RustDesktopIcons.Fence").into_iter().find(|h| {
            let r = rect(*h);
            let f = fence_cfg(&self.data, id);
            (r.left as i64 - f["x"].as_i64().unwrap_or(0)).abs() < 40 && (r.top as i64 - f["y"].as_i64().unwrap_or(0)).abs() < 40
        })
    }

    fn fence(&self, id: u64) -> Value {
        fence_cfg(&self.data, id)
    }

    fn until(&self, f: impl Fn(&Value) -> bool) -> bool {
        wait(3, || f(&saved(&self.data)))
    }
}

fn empty_spot(h: HWND) -> (i32, i32) {
    let r = rect(h);
    (r.right - 30, r.bottom - 30)
}

fn first_item(h: HWND) -> (i32, i32) {
    let (r, k) = (rect(h), unsafe { GetDpiForWindow(h) } as i32);
    (r.left + 60 * k / 96, r.top + 75 * k / 96)
}

fn fence_menu(h: HWND, path: &[&str]) -> bool {
    right_click(empty_spot(h));
    menu::open() && menu::pick(path)
}

fn has(list: &Value, v: &str) -> bool {
    list.as_array().is_some_and(|a| a.iter().any(|x| x == v))
}

fn main() {
    let w = work();
    let root = env::temp_dir().join(format!("rdi-actions-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    let dir = |id: u64| data.join("fences").join(id.to_string());
    for id in 1..=5 {
        fs::create_dir_all(dir(id)).unwrap();
    }
    fs::create_dir_all(dir(1).join("Box")).unwrap();
    fs::write(dir(3).join("move-me.txt"), "x").unwrap();
    let out_name = format!("e2e-out-{}.rdie2e", std::process::id());
    fs::write(dir(4).join(&out_name), "x").unwrap();
    fs::write(dir(5).join("delete-me.txt"), "x").unwrap();
    fs::create_dir_all(dir(6)).unwrap();
    fs::create_dir_all(dir(7).join("Deep")).unwrap();
    fs::write(dir(7).join("Deep").join("inner.txt"), "x").unwrap();
    for n in ["d1.txt", "d2.txt", "d3.txt"] {
        fs::write(dir(6).join(n), "x").unwrap();
    }
    let paste_src = root.join("paste-src.txt");
    fs::write(&paste_src, "x").unwrap();
    let fence = |id: u64, col: i32, row: i32, title: &str| {
        json!({"id": id, "x": w.left + 40 + col * 340, "y": w.top + 80 + row * 280, "w": 300, "h": 220, "auto_height": false,
               "tabs": [{"id": id, "title": title}]})
    };
    let mut roll = fence(7, 0, 2, "Roll");
    roll["rolled"] = json!(true);
    roll["h"] = json!(160);
    roll["y"] = json!(w.top + 600);
    let cfg = json!({"lang": "en", "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"),
        "peek_key": {"mods": 7, "vk": 122}, "search_key": null,
        "fences": [fence(1, 0, 0, "Main"), fence(2, 1, 0, "Other"), fence(3, 2, 0, "Mover"), fence(4, 0, 1, "Out"), fence(5, 1, 1, "Del"), fence(6, 2, 1, "Drag"), roll]});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let mut child = app(&root, &[("RDI_TOP", "1")], &[]);
    let mut t = Run { pid: child.id(), data: data.clone(), fails: vec![] };
    t.check(wait(10, || support::fences(t.pid).len() == 7), "seven fences shown");
    thread::sleep(Duration::from_millis(1000));
    let (Some(main), Some(other)) = (t.hwnd("Main"), t.hwnd("Other")) else {
        t.check(false, "fences found");
        quit(&mut child);
        std::process::exit(1);
    };

    fence_menu(main, &["Rename"]);
    thread::sleep(Duration::from_millis(300));
    chord(&[VK_CONTROL, VK_A]);
    type_text("Renamed");
    chord(&[VK_RETURN]);
    t.check(t.until(|c| c["fences"][0]["tabs"][0]["title"] == "Renamed"), "Rename changes the fence title");

    fence_menu(main, &["Sort by", "Size"]);
    t.check(t.until(|_| t.fence(1)["tabs"][0]["sort"] == "Size"), "Sort by → Size is saved");
    fence_menu(main, &["Sorting rules", "Images"]);
    t.check(t.until(|_| has(&t.fence(1)["tabs"][0]["kinds"], "Images")), "Sorting rules → Images adds the rule");

    let look_steps: [(&[&str], &str, Value); 10] = [
        (&["Appearance", "Opacity", "50%"], "alpha", json!(128)),
        (&["Appearance", "Icon size", "64 px"], "icon", json!(64)),
        (&["Appearance", "Show names"], "labels", json!(false)),
        (&["Appearance", "Name size", "Large"], "label_size", json!("Large")),
        (&["Appearance", "Icon spacing", "Wide"], "spacing", json!("Wide")),
        (&["Appearance", "Icon alignment", "Center"], "align", json!("Center")),
        (&["Appearance", "Chameleon mode"], "chameleon", json!(true)),
        (&["Behavior", "Automatic height"], "auto_height", json!(true)),
        (&["Open folders", "In File Explorer"], "browse", json!(false)),
        (&["Appearance", "Name color", "Automatic"], "label_color", Value::Null),
    ];
    for (path, field, want) in look_steps {
        fence_menu(main, path);
        let ok = t.until(|_| {
            let f = t.fence(1);
            if field == "alpha" { f[field].as_u64().is_some_and(|a| a.abs_diff(128) <= 2) } else { f[field] == want }
        });
        t.check(ok, &format!("{} sets {field} on this fence", path.join(" → ")));
    }
    t.check(t.fence(1)["custom"].as_array().is_some_and(|c| c.len() >= 8), "per-fence changes are kept as overrides");
    t.check(saved(&data)["fences"][1]["label_size"] == "Normal", "another fence is not changed");
    fence_menu(main, &["Appearance", "Color"]);
    let dialog = support::answer(t.pid, &[VK_ESCAPE]);
    t.check(dialog && wait(3, || windows(t.pid, "#32770").is_empty()), "Color… opens the color picker, Esc cancels it");
    fence_menu(main, &["Appearance", "Reset to default"]);
    t.check(
        t.until(|_| {
            let f = t.fence(1);
            f["custom"] == json!([]) && f["label_size"] == "Normal" && f["spacing"] == "Normal" && f["labels"] == true
        }),
        "Reset to default makes the fence follow the settings again",
    );

    let r0 = rect(main);
    fence_menu(main, &["Behavior", "Lock position"]);
    t.check(t.until(|_| t.fence(1)["locked"] == true), "Lock position is saved");
    drag((r0.left + 80, r0.top + 12), (r0.left + 180, r0.top + 80));
    thread::sleep(Duration::from_millis(400));
    let r1 = rect(main);
    t.check((r1.left, r1.top) == (r0.left, r0.top), "a locked fence cannot be dragged");
    fence_menu(main, &["Behavior", "Lock position"]);
    t.until(|_| t.fence(1)["locked"] == false);

    fence_menu(main, &["Behavior", "Group with", "Other"]);
    t.check(
        t.until(|_| !t.fence(1)["group"].is_null() && t.fence(1)["group"] == t.fence(2)["group"]),
        "Group with puts both fences in one group",
    );
    right_click(empty_spot(main));
    menu::open();
    menu::pick(&["Behavior", "Group with"]);
    let checked = menu::checked("Other");
    menu::close();
    t.check(checked == Some(true), "the group menu ticks the grouped fence");
    fence_menu(main, &["Behavior", "Ungroup"]);
    let ungrouped = t.until(|_| t.fence(1)["group"].is_null() && t.fence(2)["group"].is_null());
    if !ungrouped {
        println!(
            "      groups after Ungroup: {} / {}, menus open: {}",
            t.fence(1)["group"],
            t.fence(2)["group"],
            windows(0, "#32768").len()
        );
    }
    t.check(ungrouped, "Ungroup dissolves a group of two");

    for (label, field) in [("Roll up / down", "rolled"), ("Vertical title", "vertical")] {
        fence_menu(main, &["Behavior", label]);
        let on = t.until(|_| t.fence(1)[field] == true);
        thread::sleep(Duration::from_millis(600));
        let h = t.hwnd("Renamed").unwrap_or(main);
        fence_menu(h, &["Behavior", label]);
        t.check(on && t.until(|_| t.fence(1)[field] == false), &format!("{label} toggles"));
        thread::sleep(Duration::from_millis(600));
    }

    fence_menu(main, &["Tabs", "New tab"]);
    t.check(t.until(|_| t.fence(1)["tabs"].as_array().is_some_and(|a| a.len() == 2)), "New tab adds a tab");
    thread::sleep(Duration::from_millis(400));
    fence_menu(main, &["Tabs", "Delete tab"]);
    let asked = support::answer(t.pid, &[VK_RETURN]);
    t.check(asked && t.until(|_| t.fence(1)["tabs"].as_array().is_some_and(|a| a.len() == 1)), "Delete tab asks, then removes the tab");
    fence_menu(main, &["Tabs", "New portal tab"]);
    let picker = support::answer(t.pid, &[VK_ESCAPE]);
    t.check(
        picker && t.fence(1)["tabs"].as_array().is_some_and(|a| a.len() == 1),
        "New portal tab opens a folder picker; cancel adds nothing",
    );

    let before = t.fence(1)["icon"].as_i64();
    support::point(empty_spot(main));
    ctrl_wheel(true);
    t.check(t.until(|_| t.fence(1)["icon"].as_i64() > before), "Ctrl + mouse wheel enlarges the icons of that fence");

    fence_menu(main, &["Show the fence in File Explorer"]);
    let opened = poll(5, || explorer("1"));
    t.check(opened.is_some(), "Open folder shows the fence folder in Explorer");
    opened.into_iter().for_each(close);

    let n = support::fences(t.pid).len();
    fence_menu(main, &["New fence"]);
    t.check(wait(3, || support::fences(t.pid).len() == n + 1), "New fence creates a fence");
    thread::sleep(Duration::from_millis(600));
    let fresh = windows(t.pid, "RustDesktopIcons.Fence").into_iter().find(|h| ![main, other].contains(h) && t.hwnd("Fence") == Some(*h));
    if let Some(h) = fresh {
        fence_menu(h, &["Delete fence"]);
        let asked = support::answer(t.pid, &[VK_RETURN]);
        t.check(asked && wait(3, || support::fences(t.pid).len() == n), "Delete fence asks, then removes the fence");
    } else {
        t.check(false, "the new fence is found");
    }
    fence_menu(main, &["Settings"]);
    let s = poll(3, || windows(t.pid, "RustDesktopIcons.Settings").first().copied());
    t.check(s.is_some(), "Settings opens the settings window");
    s.into_iter().for_each(close);
    thread::sleep(Duration::from_millis(400));

    if let Some(h) = t.hwnd("Mover") {
        right_click(first_item(h));
        menu::open();
        menu::pick(&["Move to", "Other"]);
        t.check(
            wait(5, || dir(2).join("move-me.txt").exists() && !dir(3).join("move-me.txt").exists()),
            "Move to → another fence moves the file",
        );
    }
    let desk = support::desk::desktop();
    if let Some(h) = t.hwnd("Out") {
        right_click(first_item(h));
        menu::open();
        menu::pick(&["Move to", "Desktop"]);
        t.check(
            wait(5, || desk.join(&out_name).exists() && !dir(4).join(&out_name).exists()),
            "Move to → Desktop puts the file back on the desktop",
        );
        let _ = fs::remove_file(desk.join(&out_name));
    }
    if let Some(h) = t.hwnd("Del") {
        click(first_item(h));
        thread::sleep(Duration::from_millis(300));
        chord(&[VK_SHIFT, VK_DELETE]);
        support::answer(t.pid, &[VK_RETURN]);
        t.check(wait(5, || !dir(5).join("delete-me.txt").exists()), "Shift+Delete deletes the selected item (after Windows asks)");
    }

    t.check(clip::set_files(&[paste_src.as_path()]), "a file is put on the clipboard");
    click(empty_spot(other));
    thread::sleep(Duration::from_millis(300));
    chord(&[VK_CONTROL, VK_V]);
    let pasted =
        |d: &Path| fs::read_dir(d).into_iter().flatten().flatten().any(|e| e.file_name().to_string_lossy().starts_with("paste-src"));
    t.check(wait(5, || pasted(&dir(2))), "Ctrl+V in a fence adds the copied file (as a shortcut)");
    t.check(paste_src.exists(), "the pasted original stays where it was");

    chord(&PEEK);
    thread::sleep(Duration::from_millis(500));
    type_text("box");
    thread::sleep(Duration::from_millis(500));
    chord(&[VK_RETURN]);
    let found = poll(5, || explorer("Box"));
    t.check(found.is_some(), "Enter in the search opens the found item");
    found.into_iter().for_each(close);
    chord(&[VK_ESCAPE]);
    thread::sleep(Duration::from_millis(500));

    let main = t.hwnd("Renamed").unwrap_or(main);
    point_and_ctrl_double(first_item(main));
    let found = poll(5, || explorer("Box"));
    t.check(found.is_some(), "Ctrl+double-click on a folder opens it in Explorer");
    found.into_iter().for_each(close);
    thread::sleep(Duration::from_millis(500));
    let r = rect(main);
    double_click(first_item(main));
    thread::sleep(Duration::from_millis(800));
    t.check(explorer("Box").is_none(), "a plain double-click does not open Explorer");
    click(empty_spot(main));
    thread::sleep(Duration::from_millis(300));
    chord(&[VK_CONTROL, VK_V]);
    t.check(wait(5, || pasted(&dir(1).join("Box"))), "the folder opened inside the fence receives what is pasted there");
    click((r.left + 20, r.top + 12));
    thread::sleep(Duration::from_millis(600));
    click(first_item(main));
    thread::sleep(Duration::from_millis(300));
    chord(&[VK_F2]);
    thread::sleep(Duration::from_millis(300));
    chord(&[VK_CONTROL, VK_A]);
    type_text("Box2");
    chord(&[VK_RETURN]);
    t.check(wait(3, || dir(1).join("Box2").is_dir()), "the title-bar arrow goes back to the fence's own items");
    right_click(first_item(main));
    menu::open();
    menu::pick(&["Open in the fence"]);
    thread::sleep(Duration::from_millis(600));
    click(empty_spot(main));
    chord(&[VK_CONTROL, VK_V]);
    t.check(wait(5, || pasted(&dir(1).join("Box2"))), "Open in the fence (folder right-click) opens the folder inside the fence");
    click((r.left + 20, r.top + 12));

    if let Some(h) = windows(t.pid, "RustDesktopIcons.Fence").into_iter().find(|h| (rect(*h).top - (w.top + 600)).abs() < 40) {
        let bar = rect(h);
        let tall = || rect(h).bottom - rect(h).top > 100;
        support::point((bar.left + 150, bar.top + 15));
        let opened = wait(4, tall);
        thread::sleep(Duration::from_millis(500));
        double_click(first_item(h));
        thread::sleep(Duration::from_millis(800));
        support::point((w.right - 20, w.bottom - 20));
        let rolled = wait(5, || !tall());
        thread::sleep(Duration::from_millis(800));
        support::point((bar.left + 150, bar.top + 15));
        wait(4, tall);
        thread::sleep(Duration::from_millis(600));
        click(first_item(h));
        thread::sleep(Duration::from_millis(300));
        chord(&[VK_F2]);
        thread::sleep(Duration::from_millis(300));
        chord(&[VK_CONTROL, VK_A]);
        type_text("Deep2");
        chord(&[VK_RETURN]);
        t.check(opened && rolled && wait(3, || dir(7).join("Deep2").is_dir()), "a rolled fence that rolls up goes back to its own folder");
        support::point((w.right - 20, w.bottom - 20));
    } else {
        t.check(false, "the rolled fence is shown");
    }

    if let (Some(h), Some(del)) = (t.hwnd("Drag"), t.hwnd("Del")) {
        let (k, at) = (unsafe { GetDpiForWindow(h) } as i32, first_item(h));
        let cell = 104 * k / 96;
        drag((at.0 + cell, at.1), (at.0 - cell / 3, at.1));
        t.check(
            t.until(|_| t.fence(6)["tabs"][0]["order"].get(0) == Some(&json!("d2.txt"))),
            "dragging an item before another reorders the tab",
        );
        let r = rect(del);
        drag((at.0, at.1 + cell), ((r.left + r.right) / 2, (r.top + r.bottom) / 2));
        t.check(
            wait(5, || dir(5).join("d3.txt").exists() && !dir(6).join("d3.txt").exists()),
            "dragging an item onto another fence moves it there",
        );
        thread::sleep(Duration::from_millis(500));
        let b = rect(h);
        drag((b.right - 15, b.bottom - 15), (b.left + 8, b.top + 44 * k / 96));
        thread::sleep(Duration::from_millis(300));
        right_click(at);
        menu::open();
        menu::pick(&["Move to", "Other"]);
        let both = || ["d1.txt", "d2.txt"].iter().all(|n| dir(2).join(n).exists() && !dir(6).join(n).exists());
        t.check(wait(5, both), "a rubber-band selection moves together (Move to)");
    }
    fence_menu(other, &["Tabs", "New tab"]);
    let two = t.until(|_| t.fence(2)["tabs"].as_array().is_some_and(|a| a.len() == 2) && t.fence(2)["tab"] == 1);
    let r = rect(other);
    thread::sleep(Duration::from_millis(400));
    click((r.left + (r.right - r.left) / 4, r.top + 12));
    thread::sleep(Duration::from_millis(400));
    let n = support::fences(t.pid).len();
    fence_menu(main, &["Duplicate"]);
    let copies = |c: &Value| c["fences"].as_array().map_or(0, |a| a.iter().filter(|f| f["tabs"][0]["title"] == "Renamed").count());
    t.check(wait(3, || support::fences(t.pid).len() == n + 1) && t.until(|c| copies(c) == 2), "Duplicate adds a copy with the same tabs");
    let copy = saved(&data)["fences"]
        .as_array()
        .and_then(|a| a.iter().rev().find(|f| f["tabs"][0]["title"] == "Renamed").cloned())
        .unwrap_or_default();
    let tab = copy["tabs"][0]["id"].as_u64().unwrap_or(0);
    let empty = fs::read_dir(dir(tab)).map(|d| d.count() == 0).unwrap_or(true);
    t.check(copy["id"] != json!(1) && copy["x"] != t.fence(1)["x"] && empty, "the copy sits next to the original, without its files");
    let n = |f: &Value, k: &str| f[k].as_i64().unwrap_or(0);
    let apart = |a: &Value, b: &Value| {
        n(a, "x") + n(a, "w") <= n(b, "x")
            || n(b, "x") + n(b, "w") <= n(a, "x")
            || n(a, "y") + n(a, "h") <= n(b, "y")
            || n(b, "y") + n(b, "h") <= n(a, "y")
    };
    let others = saved(&data)["fences"].as_array().cloned().unwrap_or_default();
    t.check(others.iter().filter(|f| f["id"] != copy["id"]).all(|f| apart(f, &copy)), "the copy does not cover another fence");

    quit(&mut child);
    t.check(two && t.fence(2)["tab"] == 0, "clicking a tab header switches the tab (saved on exit)");
    let _ = fs::remove_dir_all(&root);
    if t.fails.is_empty() {
        println!("ACTIONS E2E OK");
    } else {
        println!("{} check(s) failed", t.fails.len());
        std::process::exit(1);
    }
}

fn point_and_ctrl_double(p: (i32, i32)) {
    support::point(p);
    thread::sleep(Duration::from_millis(150));
    support::hold(VK_CONTROL, true);
    for down in [true, false, true, false] {
        press(down);
    }
    support::hold(VK_CONTROL, false);
}
