mod browse;
mod drag;
mod items;
mod menus;
#[path = "../support/mod.rs"]
mod support;

use serde_json::{Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
use support::{app, fence_cfg, menu, press, quit, rect, right_click, saved, wait, windows, work};
use windows::Win32::{
    Foundation::{HWND, RECT},
    UI::{HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::*},
};

struct Run {
    pid: u32,
    root: PathBuf,
    data: PathBuf,
    w: RECT,
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

    fn dir(&self, id: u64) -> PathBuf {
        self.data.join("fences").join(id.to_string())
    }

    fn out_name(&self) -> String {
        format!("e2e-out-{}.rdie2e", std::process::id())
    }

    fn paste_src(&self) -> PathBuf {
        self.root.join("paste-src.txt")
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

fn pasted(d: &Path) -> bool {
    fs::read_dir(d).into_iter().flatten().flatten().any(|e| e.file_name().to_string_lossy().starts_with("paste-src"))
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

fn setup(t: &Run) {
    let (w, data, dir) = (t.w, &t.data, |id: u64| t.dir(id));
    for id in 1..=5 {
        fs::create_dir_all(dir(id)).unwrap();
    }
    fs::create_dir_all(dir(1).join("Box")).unwrap();
    fs::write(dir(3).join("move-me.txt"), "x").unwrap();
    fs::write(dir(4).join(t.out_name()), "x").unwrap();
    fs::write(dir(5).join("delete-me.txt"), "x").unwrap();
    fs::create_dir_all(dir(6)).unwrap();
    fs::create_dir_all(dir(7).join("Deep")).unwrap();
    fs::write(dir(7).join("Deep").join("inner.txt"), "x").unwrap();
    for n in ["d1.txt", "d2.txt", "d3.txt"] {
        fs::write(dir(6).join(n), "x").unwrap();
    }
    fs::write(t.paste_src(), "x").unwrap();
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
}

fn main() {
    let w = work();
    let root = env::temp_dir().join(format!("rdi-actions-{}", std::process::id()));
    let mut t = Run { pid: 0, data: root.join("RustDesktopIcons"), root, w, fails: vec![] };
    setup(&t);
    let mut child = app(&t.root, &[("RDI_TOP", "1")], &[]);
    t.pid = child.id();
    t.check(wait(10, || support::fences(t.pid).len() == 7), "seven fences shown");
    thread::sleep(Duration::from_millis(1000));
    let (Some(main), Some(other)) = (t.hwnd("Main"), t.hwnd("Other")) else {
        t.check(false, "fences found");
        quit(&mut child);
        std::process::exit(1);
    };

    menus::run(&mut t, main, other);
    items::run(&mut t, other);
    let main = t.hwnd("Renamed").unwrap_or(main);
    browse::run(&mut t, main);
    let two = drag::run(&mut t, main, other);

    quit(&mut child);
    t.check(two && t.fence(2)["tab"] == 0, "clicking a tab header switches the tab (saved on exit)");
    let _ = fs::remove_dir_all(&t.root);
    if t.fails.is_empty() {
        println!("ACTIONS E2E OK");
    } else {
        println!("{} check(s) failed", t.fails.len());
        std::process::exit(1);
    }
}
