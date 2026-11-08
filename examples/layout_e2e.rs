mod support;

use serde_json::{Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Child,
    thread,
    time::{Duration, Instant},
};
use support::{capture, desk, drag, fences, glide, point, press, work};
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::UI::Shell::FWF_AUTOARRANGE;
use windows::Win32::UI::{HiDpi::GetDpiForSystem, WindowsAndMessaging::*};

const TITLE: i32 = 34;
const GAP: i32 = 4;

fn s(v: i32) -> i32 {
    v * unsafe { GetDpiForSystem() }.max(96) as i32 / 96
}

fn cell() -> i32 {
    s(48 + 56)
}

fn launch(root: &Path, cfg: Value, real: bool) -> Child {
    let data = root.join("RustDesktopIcons");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let env: &[(&str, &str)] = if real { &[("RDI_REAL_DESKTOP", "1"), ("RDI_TOP", "1")] } else { &[("RDI_TOP", "1")] };
    let app = support::app(root, env, &[]);
    let t = Instant::now();
    while fences(app.id()).len() < cfg["fences"].as_array().map_or(0, |f| f.len()) && t.elapsed() < Duration::from_secs(10) {
        thread::sleep(Duration::from_millis(100));
    }
    thread::sleep(Duration::from_millis(800));
    app
}

fn by_left(pid: u32) -> Vec<RECT> {
    let mut r = fences(pid);
    r.sort_by_key(|r| (r.left, r.top));
    r
}

fn near(pid: u32, (x, y): (i32, i32)) -> RECT {
    fences(pid).into_iter().min_by_key(|r| (r.left - x).abs() + (r.top - y).abs()).unwrap_or_default()
}

fn fence(id: u64, (x, y): (i32, i32), extra: Value) -> Value {
    let mut f = json!({"id": id, "x": x, "y": y, "w": s(360), "h": s(240), "tabs": [{"id": id, "title": format!("E2E {id}")}]});
    f.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    f
}

fn base(fences: Vec<Value>) -> Value {
    json!({"auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "fences": fences})
}

fn first_item(r: &RECT) -> (i32, i32) {
    let (w, c) = (r.right - r.left, cell());
    let cols = (w / c).max(1);
    (r.left + (w - cols * c) / 2 + c / 2, r.top + s(TITLE + GAP) + c / 2)
}

fn main() {
    let shots = env::args().skip_while(|a| a != "--shots").nth(1).map(PathBuf::from);
    let w = work();
    let root = env::temp_dir().join(format!("rdi-layout-{}", std::process::id()));
    let mut saved = POINT::default();
    unsafe { GetCursorPos(&mut saved).unwrap() };
    let mut fails = vec![];
    let mut check = |ok: bool, what: String| {
        support::verdict(ok, &what, &mut fails);
    };

    let (sw, sh) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    let (ww, wh) = (w.right - w.left, w.bottom - w.top);
    let big = json!([{"full": [0, 0, sw * 2, sh * 2], "work": [w.left * 2, w.top * 2, w.left * 2 + ww * 2, w.top * 2 + wh * 2]}]);
    let mut cfg = base(vec![fence(1, (w.left * 2 + s(600), w.top * 2 + s(400)), json!({"w": s(720), "auto_height": false, "h": s(480)}))]);
    cfg["screens"] = big;
    let mut app = launch(&root.join("scale"), cfg, false);
    let r = fences(app.id()).first().copied().unwrap_or_default();
    println!("      scaled fence: {:?}", (r.left, r.top, r.right - r.left, r.bottom - r.top));
    check(
        (r.left - (w.left + s(300))).abs() <= 1 && (r.top - (w.top + s(200))).abs() <= 1,
        "a layout from a 2x larger screen is scaled at startup".into(),
    );
    check((r.right - r.left - s(360)).abs() <= 1, "fence width follows the scale".into());
    let _ = app.kill();
    let _ = app.wait();

    let data = root.join("snap").join("RustDesktopIcons");
    fs::create_dir_all(data.join("fences").join("1").join("box")).unwrap();
    fs::create_dir_all(data.join("fences").join("2")).unwrap();
    fs::write(data.join("fences").join("2").join("note.txt"), "x").unwrap();
    let (ax, ay) = (w.left + s(100), w.top + s(100));
    let free = json!({"auto_height": false});
    let mut app =
        launch(&root.join("snap"), base(vec![fence(1, (ax, ay), free.clone()), fence(2, (ax + s(600), ay + s(400)), free)]), false);
    let pid = app.id();
    let [a, b] = by_left(pid)[..] else { panic!("two fences expected") };

    let grip = (b.left + s(150), b.top + s(15));
    point(grip);
    thread::sleep(Duration::from_millis(300));
    press(true);
    thread::sleep(Duration::from_millis(150));
    let target = (grip.0 - (b.left - a.left) + s(7), grip.1);
    glide(grip, target);
    if let Some(dir) = &shots {
        let r = RECT { left: a.left - 20, top: a.top - 20, right: b.right + 20, bottom: b.bottom + 20 };
        capture(&r, &dir.join("guides.bmp"));
    }
    let lines = support::windows(pid, "RustDesktopIcons.Guide").len();
    press(false);
    thread::sleep(Duration::from_millis(400));
    let (a2, b2) = (near(pid, (a.left, a.top)), near(pid, (a.left, b.top)));
    println!("      moved fence: {:?} -> {:?}", (b.left, b.top), (b2.left, b2.top));
    check(b2.left == a2.left, "a dragged fence lines up with the left edge of another fence".into());
    check(lines >= 1, format!("a guide is shown while it lines up ({lines})"));
    check(support::windows(pid, "RustDesktopIcons.Guide").is_empty(), "guides disappear after the move".into());

    let edge = (a.right - s(2), a.top + s(120));
    drag(edge, (edge.0 + s(45), edge.1));
    thread::sleep(Duration::from_millis(400));
    let a3 = by_left(pid)[0];
    let width = a3.right - a3.left;
    println!("      resized width: {} -> {width} (cell {})", a.right - a.left, cell());
    check(width % cell() == 0, "resizing snaps the width to whole icon columns".into());

    let file = data.join("fences").join("2").join("note.txt");
    let [a4, b4] = by_left(pid)[..] else { panic!("two fences expected") };
    let (from, to) = (first_item(&b4), first_item(&a4));
    drag(from, to);
    let t = Instant::now();
    let moved = data.join("fences").join("1").join("box").join("note.txt");
    while !moved.exists() && t.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(100));
    }
    check(moved.exists() && !file.exists(), "a file dropped on a folder inside a fence moves into that folder".into());

    let _ = app.kill();
    let _ = app.wait();
    point((saved.x, saved.y));

    if env::args().any(|a| a == "--desktop") {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
        if desk::flags() & FWF_AUTOARRANGE.0 as u32 != 0 {
            println!("[SKIP] desktop auto-arrange is on, new icons cannot be placed");
        } else {
            let wide = json!({"w": w.right - w.left - s(320), "h": w.bottom - w.top, "auto_height": false});
            let mut app = launch(&root.join("desk"), base(vec![fence(1, (w.left, w.top), wide)]), true);
            let f = fences(app.id()).first().copied().unwrap_or_default();
            let path = desk::desktop().join("rdi-e2e-new.rdie2e");
            fs::write(&path, "Rust Desktop Icons e2e test file").unwrap();
            let cell = s(75);
            let covered = |(x, y): (i32, i32)| x < f.right && x + cell > f.left && y < f.bottom && y + cell > f.top;
            let spot = || desk::positions().into_iter().find(|(p, _)| p.ends_with("rdi-e2e-new.rdie2e")).map(|(_, pt)| desk::screen(pt));
            let t = Instant::now();
            let first = loop {
                if let Some(p) = spot() {
                    break Some(p);
                }
                if t.elapsed() > Duration::from_secs(5) {
                    break None;
                }
                thread::sleep(Duration::from_millis(50));
            };
            while spot().is_some_and(covered) && t.elapsed() < Duration::from_secs(10) {
                thread::sleep(Duration::from_millis(100));
            }
            println!("      new icon: first {first:?}, now {:?}, fence {:?}", spot(), (f.left, f.top, f.right, f.bottom));
            check(spot().is_some_and(|p| !covered(p)), "a new desktop file that lands under a fence moves to a visible spot".into());
            let _ = fs::remove_file(&path);
            let _ = app.kill();
            let _ = app.wait();
        }
    }
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("all layout checks passed");
    } else {
        println!("{} failed", fails.len());
        std::process::exit(1);
    }
}
