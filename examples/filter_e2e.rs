mod support;

use serde_json::json;
use std::{env, fs, thread, time::Duration};
use support::{app, chord, clip, point, press, quit, rect, type_text, wait, windows, work};
use windows::Win32::{
    Foundation::HWND,
    System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
    UI::{
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::{VK_A, VK_BACK, VK_C, VK_CONTROL, VK_ESCAPE, VK_F},
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
    sleep(400);
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
    let mut v = clip::names();
    v.sort();
    v
}

fn names(v: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = v.iter().map(|s| s.to_string()).collect();
    v.sort();
    v
}

fn main() {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    let w = work();
    let root = env::temp_dir().join(format!("rdi-filter-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    let all = ["Été.txt", "photo.jpg", "notes.txt", "report été.pdf"];
    let portal = root.join("portal");
    for (dir, list) in [(portal.clone(), &all[..]), (data.join("fences").join("2"), &["other.txt"][..])] {
        fs::create_dir_all(&dir).unwrap();
        list.iter().for_each(|n| fs::write(dir.join(n), "x").unwrap());
    }
    let at = |x: i32, y: i32| (w.left + x, w.top + y);
    let fence = |id: u64, (x, y): (i32, i32), tab: serde_json::Value| json!({"id": id, "x": x, "y": y, "w": 460, "h": 300, "tabs": [tab]});
    let cfg = json!({"lang": "en", "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"),
        "fences": [fence(1, at(40, 60), json!({"id": 1, "title": "Portal", "portal": portal})), fence(2, at(560, 60), json!({"id": 2, "title": "F2"}))]});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let mut app = app(&root, &[("RDI_TOP", "1")], &[]);
    let pid = app.id();
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| support::verdict(ok, what, &mut fails);
    let find = |x: i32, y: i32| {
        let (x, y) = at(x, y);
        windows(pid, "RustDesktopIcons.Fence").into_iter().find(|h| (rect(*h).left - x).abs() < 40 && (rect(*h).top - y).abs() < 40)
    };
    check(wait(10, || find(40, 60).is_some() && find(560, 60).is_some()), "two fences shown");
    sleep(1000);
    let (Some(f), Some(other)) = (find(40, 60), find(560, 60)) else {
        quit(&mut app);
        std::process::exit(1);
    };
    let sc = |v: i32| v * unsafe { GetDpiForWindow(f) } as i32 / 96;
    let loupe = || {
        let b = rect(f);
        (b.right - sc(16), b.top + sc(14))
    };

    check(shown(f) == names(&all), "the portal shows every icon");
    chord(&[VK_CONTROL, VK_F]);
    sleep(200);
    type_text("ETE");
    sleep(400);
    let got = shown(f);
    check(got == names(&["Été.txt", "report été.pdf"]), &format!("Ctrl+F then typing keeps matching icons, accents folded ({got:?})"));
    check(shown(other) == names(&["other.txt"]), "the other fence is not filtered");
    click_at(empty_spot(f));
    for _ in 0..3 {
        chord(&[VK_BACK]);
        sleep(150);
    }
    check(shown(f) == names(&all), "Backspace erases the query and every icon comes back");
    type_text("pho");
    sleep(300);
    check(shown(f) == names(&["photo.jpg"]), "typing again filters again");
    chord(&[VK_ESCAPE]);
    sleep(300);
    check(shown(f) == names(&all), "Escape clears the filter");
    type_text("not");
    sleep(300);
    check(shown(f) == names(&all), "typing without a filter does nothing");

    point(loupe());
    sleep(400);
    click_at(loupe());
    type_text("not");
    sleep(300);
    check(shown(f) == names(&["notes.txt"]), "a click on the portal's magnifier opens the filter");
    click_at(loupe());
    check(shown(f) == names(&all), "a second click on the magnifier closes it");
    chord(&[VK_CONTROL, VK_F]);
    type_text("zzz");
    sleep(300);
    check(shown(f).is_empty(), "nothing matches: the fence is empty");
    chord(&[VK_CONTROL, VK_F]);
    sleep(300);
    check(shown(f) == names(&all), "Ctrl+F again closes the filter");

    quit(&mut app);
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("all checks passed");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
