mod support;

use serde_json::{Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
use support::{app, chord, click, fence_cfg, fences, menu, point, quit, rect, right_click, saved, type_text, wait, windows, work};
use windows::Win32::{Foundation::HWND, UI::HiDpi::GetDpiForWindow, UI::Input::KeyboardAndMouse::*};

fn picture() -> Option<PathBuf> {
    let mut todo = vec![PathBuf::from(r"C:\Windows\Web")];
    while let Some(d) = todo.pop() {
        for e in fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                todo.push(p);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("jpg")) {
                return Some(p);
            }
        }
    }
    None
}

fn dump(dir: &Path, id: u64) -> Option<(i32, Vec<u32>)> {
    let b = fs::read(dir.join(format!("{id}.bin"))).ok()?;
    let w = i32::from_le_bytes(b.get(..4)?.try_into().ok()?);
    Some((w, b[8..].as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect()))
}

fn region(dir: &Path, id: u64, (x0, y0, x1, y1): (i32, i32, i32, i32)) -> Vec<u32> {
    dump(dir, id).map_or(vec![], |(w, px)| {
        (y0..y1).flat_map(|y| (x0..x1).map(move |x| (y * w + x) as usize)).filter_map(|i| px.get(i).copied()).collect()
    })
}

fn keep(dir: &Path, id: u64, name: &str) {
    if let Ok(to) = env::var("KEEP_DUMPS") {
        let _ = fs::create_dir_all(&to);
        let _ = fs::copy(dir.join(format!("{id}.bin")), Path::new(&to).join(format!("{name}.bin")));
    }
}

fn main() {
    let w = work();
    let tag = std::process::id();
    let root = env::temp_dir().join(format!("rdi-display-{tag}"));
    let (data, shots, portal) = (root.join("RustDesktopIcons"), root.join("dumps"), root.join("portal"));
    let dir = |id: u64| data.join("fences").join(id.to_string());
    for id in [1, 2, 3, 5, 6] {
        fs::create_dir_all(dir(id)).unwrap();
    }
    fs::create_dir_all(&shots).unwrap();
    fs::create_dir_all(portal.join("Sub")).unwrap();
    for f in ["a.pdf", "b.txt"] {
        fs::write(portal.join(f), "x").unwrap();
    }
    for f in ["k1.txt", "k2.txt", "k3.txt"] {
        fs::write(dir(5).join(f), "x").unwrap();
    }
    let pic = picture();
    if let Some(p) = &pic {
        fs::copy(p, dir(1).join("photo.jpg")).unwrap();
    }
    let fence = |id: u64, col: i32, row: i32, tabs: Value| json!({"id": id, "x": w.left + 10 + col * 350, "y": w.top + 40 + row * 270, "w": 300, "h": 220, "auto_height": false, "tabs": tabs});
    let one = |id: u64, t: &str| json!([{"id": id, "title": t}]);
    let cfg = json!({"lang": "en", "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "peek_key": null, "search_key": null,
        "fences": [fence(1, 0, 0, one(1, "Pics")), fence(2, 1, 0, one(2, "Title")), fence(3, 2, 0, one(3, "Wall")),
            fence(4, 0, 1, json!([{"id": 4, "title": "Portal", "portal": portal}])), fence(5, 1, 1, json!([{"id": 5, "title": "Keys"}, {"id": 6, "title": "Second"}]))]});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let mut child = app(&root, &[("RDI_TOP", "1"), ("RDI_DUMP", shots.to_str().unwrap())], &[]);
    let pid = child.id();
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| {
        support::verdict(ok, what, &mut fails);
    };
    let until = |f: &dyn Fn(&Value) -> bool| wait(4, || f(&saved(&data)));
    let hwnd = |id: u64| {
        let f = fence_cfg(&data, id);
        windows(pid, "RustDesktopIcons.Fence")
            .into_iter()
            .find(|h| {
                (rect(*h).left as i64 - f["x"].as_i64().unwrap_or(0)).abs() < 40
                    && (rect(*h).top as i64 - f["y"].as_i64().unwrap_or(0)).abs() < 40
            })
            .unwrap_or_default()
    };
    let k = |h: HWND| unsafe { GetDpiForWindow(h) }.max(96) as i32;
    let item = |h: HWND, i: i32| {
        let r = rect(h);
        (r.left + (60 + 104 * i) * k(h) / 96, r.top + 75 * k(h) / 96)
    };
    let fence_menu = |h: HWND, path: &[&str]| {
        let r = rect(h);
        right_click((r.right - 30, r.bottom - 30));
        menu::open() && menu::pick(path)
    };
    let away = || point((w.right - 20, w.bottom - 20));
    check(wait(10, || fences(pid).len() == 5), "five fences shown");
    thread::sleep(Duration::from_millis(1500));

    let s = |v: i32| v * k(hwnd(1)) / 96;
    let bar = (s(10), s(4), s(290), s(30));
    let drawn = |id: u64| region(&shots, id, bar).iter().any(|p| p >> 24 != 0);
    check(wait(3, || drawn(2) && drawn(3)), "empty fences are drawn at startup");
    let icon_box = (s(30), s(44), s(80), s(94));
    let with_thumb = region(&shots, 1, icon_box);
    keep(&shots, 1, "thumb_on");
    fence_menu(hwnd(1), &["Appearance", "Show thumbnails"]);
    away();
    let off = until(&|_| fence_cfg(&data, 1)["thumbs"] == false);
    let changed = wait(4, || region(&shots, 1, icon_box) != with_thumb);
    keep(&shots, 1, "thumb_off");
    check(pic.is_some() && off && changed, "a picture shows a thumbnail that turning thumbnails off replaces by its icon");

    let before = region(&shots, 2, bar);
    fence_menu(hwnd(2), &["Appearance", "Title bar", "Title alignment", "Left"]);
    away();
    thread::sleep(Duration::from_millis(700));
    keep(&shots, 2, "title_left");
    check(
        until(&|_| fence_cfg(&data, 2)["title_align"] == "Left") && wait(3, || region(&shots, 2, bar) != before),
        "Title alignment moves the title",
    );
    let before = region(&shots, 2, bar);
    fence_menu(hwnd(2), &["Appearance", "Title bar", "Title size", "Large"]);
    away();
    thread::sleep(Duration::from_millis(700));
    keep(&shots, 2, "title_left_large");
    check(
        until(&|_| fence_cfg(&data, 2)["title_size"] == "Large") && wait(3, || region(&shots, 2, bar) != before),
        "Title size changes the title",
    );
    fence_menu(hwnd(2), &["Appearance", "Title bar", "Show the bar on hover only"]);
    away();
    let hidden = wait(4, || region(&shots, 2, bar).iter().all(|p| p >> 24 == 0));
    keep(&shots, 2, "bar_hidden");
    let r = rect(hwnd(2));
    point(((r.left + r.right) / 2, r.bottom - s(40)));
    let shown = wait(4, || region(&shots, 2, bar).iter().any(|p| p >> 24 != 0));
    keep(&shots, 2, "bar_shown");
    check(until(&|_| fence_cfg(&data, 2)["bar_hover"] == true) && hidden && shown, "a bar shown on hover only hides when the mouse leaves");
    away();

    let body = (s(150), s(120), s(290), s(200));
    let before = region(&shots, 3, body);
    keep(&shots, 3, "wall_before");
    fence_menu(hwnd(3), &["Appearance", "Color from the wallpaper"]);
    away();
    thread::sleep(Duration::from_millis(700));
    keep(&shots, 3, "wall_after");
    check(
        until(&|_| fence_cfg(&data, 3)["wallpaper"] == true) && wait(3, || region(&shots, 3, body) != before),
        "Color from the wallpaper recolors the fence",
    );

    fence_menu(hwnd(4), &["Tabs", "Filter files"]);
    thread::sleep(Duration::from_millis(400));
    chord(&[VK_CONTROL, VK_A]);
    type_text("*.pdf");
    chord(&[VK_RETURN]);
    let saved_filter = until(&|_| fence_cfg(&data, 4)["tabs"][0]["filter"] == "*.pdf");
    thread::sleep(Duration::from_millis(800));
    click(item(hwnd(4), 0));
    chord(&[VK_CONTROL, VK_A]);
    right_click(item(hwnd(4), 1));
    menu::open();
    menu::pick(&["Move to", "Pics"]);
    let moved = wait(4, || dir(1).join("a.pdf").exists());
    if !(saved_filter && moved && portal.join("b.txt").exists()) {
        let left: Vec<String> =
            fs::read_dir(&portal).into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        let pics: Vec<String> =
            fs::read_dir(dir(1)).into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        println!("      filter saved {saved_filter} ({}), portal {left:?}, pics {pics:?}", fence_cfg(&data, 4)["tabs"][0]["filter"]);
    }
    check(saved_filter && moved && portal.join("b.txt").exists(), "a filtered portal only shows matching files (and its folders)");

    click(item(hwnd(5), 0));
    chord(&[VK_RIGHT]);
    chord(&[VK_F2]);
    thread::sleep(Duration::from_millis(300));
    chord(&[VK_CONTROL, VK_A]);
    type_text("nav-ok");
    chord(&[VK_RETURN]);
    let named = |n: &str| dir(5).join(n).exists();
    check(
        wait(3, || !named("k2.txt") && (named("nav-ok") || named("nav-ok.txt")) && named("k1.txt")),
        "the arrow keys move the selection to the next icon",
    );
    click(item(hwnd(5), 0));
    away();
    thread::sleep(Duration::from_millis(500));
    let full = region(&shots, 5, icon_box);
    chord(&[VK_CONTROL, VK_TAB]);
    check(wait(3, || region(&shots, 5, icon_box) != full), "switching to an empty tab redraws the fence");

    quit(&mut child);
    check(fence_cfg(&data, 5)["tab"] == 1, "Ctrl+Tab switches to the next tab");
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("DISPLAY E2E OK");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
