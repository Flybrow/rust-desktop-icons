mod fixed;
mod rolled;
mod side;
#[path = "../support/mod.rs"]
mod support;

use serde_json::json;
use std::{
    env, fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
use support::{app, capture, chord, clip, point, press, quit, rect, shortcut, wait, windows, work};
use windows::Win32::{
    Foundation::HWND,
    System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
    UI::{
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::{VK_A, VK_C, VK_CONTROL, VK_HOME, VK_RETURN},
    },
};

fn sleep(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

fn click_at(p: (i32, i32)) {
    point(p);
    sleep(150);
    press(true);
    press(false);
    sleep(500);
}

fn double_at(p: (i32, i32)) {
    point(p);
    sleep(150);
    for _ in 0..2 {
        press(true);
        press(false);
        sleep(60);
    }
    sleep(600);
}

fn empty_spot(h: HWND) -> (i32, i32) {
    let b = rect(h);
    (b.right - 20, b.bottom - 20)
}

fn shown(h: HWND) -> Vec<String> {
    clip::clear();
    click_at(empty_spot(h));
    chord(&[VK_CONTROL, VK_A]);
    sleep(150);
    chord(&[VK_CONTROL, VK_C]);
    sleep(300);
    clip::names()
}

fn open_first(h: HWND) {
    click_at(empty_spot(h));
    chord(&[VK_HOME]);
    sleep(150);
    chord(&[VK_RETURN]);
    sleep(800);
}

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn files(dir: &Path, list: &[&str]) {
    fs::create_dir_all(dir).unwrap();
    for f in list {
        if f.ends_with(".txt") {
            fs::write(dir.join(f), "x").unwrap();
        } else {
            fs::create_dir_all(dir.join(f)).unwrap();
        }
    }
}

struct Ctx {
    pid: u32,
    data: PathBuf,
    shots: Option<PathBuf>,
    k: i32,
    fails: Vec<String>,
}

impl Ctx {
    fn check(&mut self, ok: bool, what: &str) {
        support::verdict(ok, what, &mut self.fails);
    }

    fn dir(&self, id: u64) -> PathBuf {
        self.data.join("fences").join(id.to_string())
    }

    fn sc(&self, v: i32) -> i32 {
        v * self.k / 96
    }

    fn bar(&self, h: HWND, along: i32) -> (i32, i32) {
        let b = rect(h);
        (b.left + along.min(b.right - b.left - self.sc(10)), b.top + self.sc(12))
    }

    fn shot(&self, h: HWND, name: &str) {
        if let Some(d) = &self.shots {
            capture(&rect(h), &d.join(format!("browse_e2e-{name}.bmp")));
        }
    }
}

fn main() {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    let shots = env::args().skip_while(|a| a != "--shots").nth(1).map(PathBuf::from);
    let w = work();
    let root = env::temp_dir().join(format!("rdi-browse-{}", std::process::id()));
    let mut c = Ctx { pid: 0, data: root.join("RustDesktopIcons"), shots, k: 0, fails: vec![] };
    let (data, dir) = (c.data.clone(), |id: u64| c.dir(id));
    files(&dir(1), &["one.txt"]);
    files(&dir(2), &["two.txt"]);
    files(&dir(3), &["A", "zhome.txt"]);
    files(&dir(3).join("A"), &["B", "za.txt"]);
    files(&dir(3).join("A").join("B"), &["deep.txt"]);
    files(&dir(4), &["R", "zhome4.txt"]);
    files(&dir(4).join("R"), &["r.txt"]);
    files(&dir(5), &["V", "zhome5.txt"]);
    files(&dir(5).join("V"), &["v.txt"]);
    files(&dir(6), &["zhome6.txt"]);
    files(&root.join("linked"), &["in.txt"]);
    shortcut(&root.join("linked"), &dir(6).join("alink.lnk"));
    let at = |x: i32, y: i32| (w.left + x, w.top + y);
    let fence = |id: u64, (x, y): (i32, i32), extra: serde_json::Value| {
        let mut f = json!({"id": id, "x": x, "y": y, "w": 420, "h": 280, "tabs": [{"id": id, "title": format!("F{id}")}]});
        f.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        f
    };
    let tabs = json!({"tab": 2, "tabs": [{"id": 1, "title": "One"}, {"id": 2, "title": "Two"}, {"id": 3, "title": "Three"}]});
    let cfg = json!({"lang": "en", "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "look": {"browse": true},
        "fences": [fence(1, at(40, 60), tabs), fence(4, at(560, 60), json!({"rolled": true})), fence(5, at(40, 460), json!({"vertical": true, "home": true})),
            fence(6, at(560, 460), json!({}))]});
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let mut app = app(&root, &[("RDI_TOP", "1")], &[]);
    let pid = app.id();
    c.pid = pid;
    let find = |x: i32, y: i32| {
        let (x, y) = at(x, y);
        windows(pid, "RustDesktopIcons.Fence").into_iter().find(|h| (rect(*h).left - x).abs() < 40 && (rect(*h).top - y).abs() < 40)
    };
    let all = || [find(40, 60), find(560, 60), find(40, 460), find(560, 460)];
    c.check(wait(10, || all().iter().all(Option::is_some)), "four fences shown");
    sleep(1000);
    let [Some(fixed), Some(rolled), Some(side), Some(linking)] = all() else {
        quit(&mut app);
        std::process::exit(1);
    };
    c.k = unsafe { GetDpiForWindow(fixed) } as i32;
    fixed::run(&mut c, fixed);
    rolled::run(&mut c, rolled);
    side::run(&mut c, side, linking);

    quit(&mut app);
    let _ = fs::remove_dir_all(&root);
    if c.fails.is_empty() {
        println!("all checks passed");
    } else {
        println!("{} check(s) failed", c.fails.len());
        std::process::exit(1);
    }
}
