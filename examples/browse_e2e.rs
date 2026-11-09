mod support;

use serde_json::json;
use std::{env, fs, path::Path, thread, time::Duration};
use support::{app, capture, chord, clip, fence_cfg, glide, point, press, quit, rect, wait, windows, work};
use windows::Win32::{
    Foundation::HWND,
    System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
    UI::{
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::{VK_A, VK_BACK, VK_C, VK_CONTROL, VK_HOME, VK_RETURN, VK_SHIFT, VK_TAB},
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

fn main() {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    let shots = env::args().skip_while(|a| a != "--shots").nth(1).map(std::path::PathBuf::from);
    let w = work();
    let root = env::temp_dir().join(format!("rdi-browse-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    let dir = |id: u64| data.join("fences").join(id.to_string());
    files(&dir(1), &["one.txt"]);
    files(&dir(2), &["two.txt"]);
    files(&dir(3), &["A", "zhome.txt"]);
    files(&dir(3).join("A"), &["B", "za.txt"]);
    files(&dir(3).join("A").join("B"), &["deep.txt"]);
    files(&dir(4), &["R", "zhome4.txt"]);
    files(&dir(4).join("R"), &["r.txt"]);
    files(&dir(5), &["V", "zhome5.txt"]);
    files(&dir(5).join("V"), &["v.txt"]);
    let at = |x: i32, y: i32| (w.left + x, w.top + y);
    let fence = |id: u64, (x, y): (i32, i32), extra: serde_json::Value| {
        let mut f = json!({"id": id, "x": x, "y": y, "w": 420, "h": 280, "tabs": [{"id": id, "title": format!("F{id}")}]});
        f.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        f
    };
    let tabs = json!({"tab": 2, "tabs": [{"id": 1, "title": "One"}, {"id": 2, "title": "Two"}, {"id": 3, "title": "Three"}]});
    let cfg = json!({"auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "look": {"browse": true},
        "fences": [fence(1, at(40, 60), tabs), fence(4, at(560, 60), json!({"rolled": true})), fence(5, at(40, 460), json!({"vertical": true}))]});
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let mut app = app(&root, &[("RDI_TOP", "1")], &[]);
    let pid = app.id();
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| support::verdict(ok, what, &mut fails);
    let find = |x: i32, y: i32| {
        let (x, y) = at(x, y);
        windows(pid, "RustDesktopIcons.Fence").into_iter().find(|h| (rect(*h).left - x).abs() < 40 && (rect(*h).top - y).abs() < 40)
    };
    check(wait(10, || find(40, 60).is_some() && find(560, 60).is_some() && find(40, 460).is_some()), "three fences shown");
    sleep(1000);
    let (Some(fixed), Some(rolled), Some(side)) = (find(40, 60), find(560, 60), find(40, 460)) else {
        quit(&mut app);
        std::process::exit(1);
    };
    let k = unsafe { GetDpiForWindow(fixed) } as i32;
    let sc = |v: i32| v * k / 96;
    let bar = |h: HWND, along: i32| {
        let b = rect(h);
        (b.left + along.min(b.right - b.left - sc(10)), b.top + sc(12))
    };
    let shot = |h: HWND, name: &str| {
        if let Some(d) = &shots {
            capture(&rect(h), &d.join(format!("browse_e2e-{name}.bmp")));
        }
    };
    let home3 = names(&["A", "zhome.txt"]);

    let first = shown(fixed);
    check(first == home3, &format!("the fixed fence shows its tab ({first:?})"));
    open_first(fixed);
    check(shown(fixed) == names(&["B", "za.txt"]), "Enter on a folder opens it inside the fence");
    open_first(fixed);
    check(shown(fixed) == names(&["deep.txt"]), "a folder inside it opens too");
    shot(fixed, "deep");
    let width = rect(fixed).right - rect(fixed).left;
    for x in (sc(20)..width - sc(20)).rev().step_by(12) {
        point(bar(fixed, x));
        sleep(10);
    }
    shot(fixed, "arrow_hovered");
    double_at(bar(fixed, sc(30)));
    for x in [40, 60, 80, 60, 30] {
        point(bar(fixed, sc(x)));
        sleep(60);
    }
    point((rect(fixed).left + sc(60), rect(fixed).top + sc(80)));
    sleep(300);
    check(shown(fixed) == home3, "after sliding along the bar, a double click on the back arrow goes up two levels and stays on that tab");
    check(fence_cfg(&data, 1)["rolled"] != json!(true), "a double click on the back arrow never rolls the fence up");
    open_first(fixed);
    click_at(bar(fixed, width / 2));
    check(shown(fixed) == names(&["two.txt"]), "clicking another tab while in a folder shows that tab");
    click_at(bar(fixed, width - sc(30)));
    check(shown(fixed) == home3, "the tab left in a folder comes back to its own items");
    open_first(fixed);
    chord(&[VK_CONTROL, VK_TAB]);
    sleep(600);
    check(shown(fixed) == names(&["one.txt"]), "Ctrl+Tab while in a folder goes to the next tab");
    chord(&[VK_CONTROL, VK_SHIFT, VK_TAB]);
    sleep(600);
    check(shown(fixed) == home3, "Ctrl+Shift+Tab comes back to the tab's own items");
    open_first(fixed);
    open_first(fixed);
    chord(&[VK_BACK]);
    sleep(600);
    check(shown(fixed) == names(&["B", "za.txt"]), "Backspace goes up one level");
    fs::remove_dir_all(dir(3).join("A")).unwrap();
    sleep(1500);
    check(shown(fixed).is_empty(), "a folder deleted while open shows empty");
    click_at(bar(fixed, sc(30)));
    check(shown(fixed) == names(&["zhome.txt"]), "the back arrow still works after the folder is deleted");

    let r0 = rect(rolled);
    let away = (r0.left + 100, r0.bottom + 450);
    glide(away, bar(rolled, sc(150)));
    sleep(900);
    let b = rect(rolled);
    check(b.bottom - b.top > r0.bottom - r0.top + sc(60), "the rolled fence opens on hover");
    check(shown(rolled) == names(&["R", "zhome4.txt"]), "the rolled fence shows its items");
    open_first(rolled);
    check(shown(rolled) == names(&["r.txt"]), "a folder opens inside the rolled fence");
    glide(empty_spot(rolled), bar(rolled, sc(30)));
    sleep(500);
    check(
        rect(rolled).bottom - rect(rolled).top > r0.bottom - r0.top + sc(60),
        "the rolled fence stays open while the mouse goes to the back arrow",
    );
    shot(rolled, "arrow");
    click_at(bar(rolled, sc(30)));
    check(shown(rolled) == names(&["R", "zhome4.txt"]), "the back arrow works in a rolled fence");
    open_first(rolled);
    double_at(bar(rolled, sc(30)));
    check(shown(rolled) == names(&["R", "zhome4.txt"]), "a double click on the back arrow of a rolled fence goes back");
    check(fence_cfg(&data, 4)["rolled"] == json!(true), "the fence stays a rolled fence");
    open_first(rolled);
    glide(bar(rolled, sc(150)), away);
    check(
        wait(5, || rect(rolled).bottom - rect(rolled).top < r0.bottom - r0.top + sc(10)),
        "the rolled fence rolls up when the mouse leaves",
    );
    glide(away, bar(rolled, sc(150)));
    sleep(900);
    check(shown(rolled) == names(&["R", "zhome4.txt"]), "rolling up brings the fence back to its own items");
    point(away);
    sleep(800);

    check(shown(side) == names(&["V", "zhome5.txt"]), "the side-bar fence shows its items");
    open_first(side);
    check(shown(side) == names(&["v.txt"]), "a folder opens in a fence with its bar on the side");
    shot(side, "folder");
    let s = rect(side);
    click_at((s.left + sc(12), s.top + sc(30)));
    check(shown(side) == names(&["V", "zhome5.txt"]), "the start of a side bar is the back arrow");

    quit(&mut app);
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("all checks passed");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
