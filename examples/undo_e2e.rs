mod support;

use serde_json::{Value, json};
use std::{env, fs, thread, time::Duration};
use support::{app, chord, click, drag, fence_cfg, fences, menu, quit, rect, right_click, saved, type_text, wait, windows, work};
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::{
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::*,
        WindowsAndMessaging::{PostMessageW, WM_APP, WM_RBUTTONUP},
    },
};

fn main() {
    let w = work();
    let tag = std::process::id();
    let root = env::temp_dir().join(format!("rdi-undo-{tag}"));
    let data = root.join("RustDesktopIcons");
    let dir = |id: u64| data.join("fences").join(id.to_string());
    for id in 1..=3 {
        fs::create_dir_all(dir(id)).unwrap();
    }
    let gone = format!("undo-{tag}.rdie2e");
    fs::write(dir(1).join("move-me.txt"), "x").unwrap();
    fs::write(dir(3).join(&gone), "x").unwrap();
    let fence = |id: u64, x: i32, title: &str| json!({"id": id, "x": w.left + x, "y": w.top + 100, "w": 300, "h": 220, "auto_height": false, "tabs": [{"id": id, "title": title}]});
    let cfg = json!({"lang": "en", "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "peek_key": null, "search_key": null,
        "fences": [fence(1, 20, "One"), fence(2, 360, "Two"), fence(3, 700, "Three")]});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let mut child = app(&root, &[("RDI_TOP", "1")], &[]);
    let pid = child.id();
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| {
        support::verdict(ok, what, &mut fails);
    };
    let until = |f: &dyn Fn(&Value) -> bool| wait(4, || f(&saved(&data)));
    let hwnd = |x: i32| windows(pid, "RustDesktopIcons.Fence").into_iter().find(|h| (rect(*h).left - (w.left + x)).abs() < 60);
    let item = |h: HWND| {
        let (r, k) = (rect(h), unsafe { GetDpiForWindow(h) } as i32);
        (r.left + 60 * k / 96, r.top + 75 * k / 96)
    };
    let fence_menu = |h: HWND, path: &[&str]| {
        let r = rect(h);
        right_click((r.right - 30, r.bottom - 30));
        menu::open() && menu::pick(path)
    };
    check(wait(10, || fences(pid).len() == 3), "three fences shown");
    thread::sleep(Duration::from_millis(1000));
    let one = || hwnd(20).unwrap_or_default();
    let three = || hwnd(700).unwrap_or_default();

    right_click((rect(one()).right - 30, rect(one()).bottom - 30));
    menu::open();
    let none = !menu::entries().iter().any(|e| e.starts_with("Undo"));
    menu::close();
    check(none, "no Undo entry before anything was done");

    let r = rect(one());
    drag((r.left + 80, r.top + 12), (r.left + 180, r.top + 72));
    check(until(&|c| c["fences"][0]["x"] != json!(w.left + 20)), "a moved fence saves its new place");
    fence_menu(three(), &["Undo"]);
    check(until(&|c| c["fences"][0]["x"] == json!(w.left + 20)) && wait(3, || hwnd(20).is_some()), "Undo puts the fence back");

    right_click(item(one()));
    menu::open();
    menu::pick(&["Move to", "Two"]);
    check(wait(4, || dir(2).join("move-me.txt").exists()), "Move to moves the file");
    fence_menu(three(), &["Undo"]);
    check(wait(4, || dir(1).join("move-me.txt").exists() && !dir(2).join("move-me.txt").exists()), "Undo brings the moved file back");

    let desk = support::desk::desktop();
    fence_menu(three(), &["Delete fence"]);
    support::answer(pid, &[VK_RETURN]);
    check(wait(4, || fences(pid).len() == 2 && desk.join(&gone).exists()), "deleting a fence puts its file on the desktop");
    fence_menu(one(), &["Undo"]);
    check(
        wait(5, || fences(pid).len() == 3 && dir(3).join(&gone).exists() && !desk.join(&gone).exists()),
        "Undo brings the fence back with its file",
    );
    thread::sleep(Duration::from_millis(600));

    fence_menu(one(), &["Rename"]);
    thread::sleep(Duration::from_millis(300));
    chord(&[VK_CONTROL, VK_A]);
    type_text("Renamed");
    chord(&[VK_RETURN]);
    let renamed = until(&|c| c["fences"][0]["tabs"][0]["title"] == "Renamed");
    click((rect(one()).right - 30, rect(one()).bottom - 30));
    chord(&[VK_CONTROL, VK_Z]);
    check(renamed && until(&|c| c["fences"][0]["tabs"][0]["title"] == "One"), "Ctrl+Z in a fence undoes a rename");

    fence_menu(one(), &["Duplicate"]);
    let dup = wait(4, || fences(pid).len() == 4);
    fence_menu(one(), &["Undo"]);
    check(dup && wait(4, || fences(pid).len() == 3), "Undo removes a duplicated fence");

    let tray = support::windows_all(pid, "RustDesktopIcons.Tray.dev");
    fence_menu(one(), &["Appearance", "Icon size", "64 px"]);
    let big = until(&|_| fence_cfg(&data, 1)["icon"] == 64);
    let c = w.left + (w.right - w.left) / 2;
    support::point((c, w.top + (w.bottom - w.top) / 2));
    for t in &tray {
        let _ = unsafe { PostMessageW(Some(*t), WM_APP + 1, WPARAM(0), LPARAM(WM_RBUTTONUP as isize)) };
    }
    menu::open();
    menu::pick(&["Undo"]);
    check(big && until(&|_| fence_cfg(&data, 1)["icon"] == 48), "the tray menu's Undo reverts a look change");

    quit(&mut child);
    let _ = fs::remove_file(desk.join(&gone));
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("UNDO E2E OK");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
