mod appearance;
mod behavior;
mod general;
mod rules;
#[path = "../support/mod.rs"]
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

fn f(c: &Value, i: usize, k: &str) -> Value {
    c["fences"][i][k].clone()
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
    appearance::icons(&mut u);
    appearance::look(&mut u);
    appearance::bar(&mut u);
    behavior::behave(&mut u);
    behavior::desk(&mut u);
    behavior::keys(&mut u);
    rules::rules(&mut u);
    rules::privacy(&mut u);
    general::general(&mut u, &export);

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
