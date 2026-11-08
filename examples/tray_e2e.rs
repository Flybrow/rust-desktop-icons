mod support;

use serde_json::{Value, json};
use std::{env, fs, thread, time::Duration};
use support::{app, chord, drag, fences, menu, point, poll, saved, type_text, wait, windows, work};
use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{VK_A, VK_CONTROL, VK_ESCAPE, VK_RETURN},
        WindowsAndMessaging::{PostMessageW, WM_APP, WM_LBUTTONDBLCLK, WM_RBUTTONUP},
    },
};

const WM_TRAY: u32 = WM_APP + 1;

fn tray(pid: u32, event: u32) {
    let w = work();
    point(((w.left + w.right) / 2, (w.top + w.bottom) / 2));
    for h in support::windows_all(pid, "RustDesktopIcons.Tray.dev") {
        let _ = unsafe { PostMessageW(Some(h), WM_TRAY, WPARAM(0), LPARAM(event as isize)) };
    }
    thread::sleep(Duration::from_millis(400));
}

fn tray_menu(pid: u32, path: &[&str]) -> bool {
    tray(pid, WM_RBUTTONUP);
    menu::open() && menu::pick(path)
}

fn main() {
    let w = work();
    let root = env::temp_dir().join(format!("rdi-tray-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    fs::create_dir_all(data.join("fences").join("1")).unwrap();
    let (x0, y0) = (w.left + 200, w.top + 200);
    let cfg = json!({"lang": "en", "auto_update": false, "auto_sort": false, "autostart": true, "version": env!("CARGO_PKG_VERSION"),
        "peek_key": null, "search_key": null,
        "fences": [{"id": 1, "x": x0, "y": y0, "w": 300, "h": 200, "tabs": [{"id": 1, "title": "Tray"}]}]});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let mut child = app(&root, &[("RDI_TOP", "1")], &[]);
    let pid = child.id();
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| {
        support::verdict(ok, what, &mut fails);
    };
    let until = |f: &dyn Fn(&Value) -> bool| wait(3, || f(&saved(&data)));
    check(wait(10, || fences(pid).len() == 1), "fence shown");
    thread::sleep(Duration::from_millis(800));

    tray_menu(pid, &["Layouts", "Save current layout"]);
    check(until(&|c| c["snapshots"].as_array().is_some_and(|a| a.len() == 1)), "Layouts → Save current layout adds a layout");
    let name = saved(&data)["snapshots"][0]["name"].as_str().unwrap_or("").to_string();
    let r = fences(pid)[0];
    drag((r.left + 80, r.top + 12), (r.left + 380, r.top + 212));
    thread::sleep(Duration::from_millis(500));
    let moved = until(&|c| c["fences"][0]["x"] != json!(x0));
    tray_menu(pid, &["Layouts", &name, "Restore"]);
    check(
        moved && until(&|c| c["fences"][0]["x"] == json!(x0) && c["fences"][0]["y"] == json!(y0)) && wait(3, || fences(pid)[0].left == x0),
        "Restore puts the fences back where the layout was saved",
    );
    tray_menu(pid, &["Layouts", &name, "Rename"]);
    thread::sleep(Duration::from_millis(400));
    chord(&[VK_CONTROL, VK_A]);
    type_text("Work");
    chord(&[VK_RETURN]);
    check(until(&|c| c["snapshots"][0]["name"] == "Work"), "a layout can be renamed");
    tray_menu(pid, &["Layouts", "Work", "Delete"]);
    check(until(&|c| c["snapshots"].as_array().is_none_or(|a| a.is_empty())), "a layout can be deleted");

    tray_menu(pid, &["Search"]);
    check(wait(3, || windows(pid, "RustDesktopIcons.Search").len() == 1), "Search… opens the search");
    chord(&[VK_ESCAPE]);
    check(wait(3, || windows(pid, "RustDesktopIcons.Search").is_empty()), "Esc closes it");
    tray_menu(pid, &["Settings"]);
    let s = poll(3, || windows(pid, "RustDesktopIcons.Settings").first().copied());
    check(s.is_some(), "Settings… opens the settings");
    s.into_iter().for_each(support::close);
    thread::sleep(Duration::from_millis(400));
    tray(pid, WM_LBUTTONDBLCLK);
    let s = poll(3, || windows(pid, "RustDesktopIcons.Settings").first().copied());
    check(s.is_some(), "double-clicking the tray icon opens the settings");
    s.into_iter().for_each(support::close);
    thread::sleep(Duration::from_millis(400));
    tray_menu(pid, &["New fence"]);
    check(wait(3, || fences(pid).len() == 2), "New fence creates a fence");
    tray_menu(pid, &["Language", "Français"]);
    let fr = until(&|c| c["lang"] == "fr");
    tray_menu(pid, &["Langue", "English"]);
    check(fr && until(&|c| c["lang"] == "en"), "Language switches the app language (and back)");
    tray_menu(pid, &["Automatic updates"]);
    check(until(&|c| c["auto_update"] == true), "Automatic updates toggles");
    tray_menu(pid, &["Start with Windows"]);
    check(until(&|c| c["autostart"] == false), "Start with Windows toggles");
    tray_menu(pid, &["About"]);
    let about = support::answer(pid, &[VK_RETURN]);
    check(about && wait(3, || windows(pid, "#32770").is_empty()), "About shows the version, OK closes it");
    tray_menu(pid, &["Quit"]);
    let mut quit = false;
    for _ in 0..50 {
        if child.try_wait().ok().flatten().is_some() {
            quit = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    check(quit, "Quit closes the app");
    if !quit {
        support::quit(&mut child);
    }
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("TRAY E2E OK");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
