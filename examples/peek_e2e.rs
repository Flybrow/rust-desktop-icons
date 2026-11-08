mod support;

use serde_json::json;
use std::{env, fs, path::PathBuf, thread, time::Duration};
use support::{app, capture, chord, desk, double_click, fences, quit, type_text, wait, windows, work};
use windows::Win32::Foundation::RECT;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_ESCAPE, VK_F11, VK_F12, VK_MENU, VK_SHIFT};

const PEEK: [windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY; 4] = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_F11];
const SEARCH: [windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY; 4] = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_F12];
use windows::Win32::UI::WindowsAndMessaging::*;

fn topmost(pid: u32) -> bool {
    let f = support::windows(pid, "RustDesktopIcons.Fence");
    !f.is_empty() && f.iter().all(|h| unsafe { GetWindowLongPtrW(*h, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST.0 != 0)
}

fn main() {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    let shots = env::args().skip_while(|a| a != "--shots").nth(1).map(PathBuf::from);
    let w = work();
    let root = env::temp_dir().join(format!("rdi-peek-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    for (id, files) in [(1, vec!["Photo été.jpg", "notes.txt"]), (2, vec!["rapport.docx"]), (3, vec!["budget.xlsx"])] {
        let dir = data.join("fences").join(id.to_string());
        fs::create_dir_all(&dir).unwrap();
        files.iter().for_each(|f| fs::write(dir.join(f), "x").unwrap());
    }
    let tabs = json!([{"id": 2, "title": "Bureau"}, {"id": 3, "title": "Compta"}]);
    let cfg = json!({"auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "dblclick_hide": true,
        "peek_key": {"mods": 7, "vk": 122}, "search_key": {"mods": 7, "vk": 123}, "fences": [
        {"id": 1, "x": w.left + 200, "y": w.top + 150, "w": 360, "h": 240, "tabs": [{"id": 1, "title": "Photos"}]},
        {"id": 2, "x": w.left + 650, "y": w.top + 150, "w": 360, "h": 240, "labels": false, "tabs": tabs}
    ]});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let mut app = app(&root, &[("RDI_REAL_DESKTOP", "1")], &[]);
    let pid = app.id();
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| {
        support::verdict(ok, what, &mut fails);
    };
    check(wait(10, || fences(pid).len() == 2), "two fences shown");
    thread::sleep(Duration::from_millis(800));

    chord(&PEEK);
    check(wait(3, || topmost(pid)), "the peek shortcut brings the fences above the windows");
    check(windows(pid, "RustDesktopIcons.Overlay").len() == 1, "a dim layer covers the rest of the screen");
    type_text("ete");
    check(wait(3, || windows(pid, "RustDesktopIcons.Search").len() == 1), "typing while peeking opens the search");
    thread::sleep(Duration::from_millis(400));
    if let Some(dir) = &shots {
        capture(&RECT { left: w.left, top: w.top, right: w.right, bottom: w.bottom }, &dir.join("peek_search.bmp"));
    }
    chord(&[VK_ESCAPE]);
    check(wait(3, || windows(pid, "RustDesktopIcons.Search").is_empty()), "Esc closes the search");
    if !topmost(pid) {
        chord(&PEEK);
        wait(3, || topmost(pid));
    }
    chord(&[VK_ESCAPE]);
    check(wait(3, || !topmost(pid) && windows(pid, "RustDesktopIcons.Overlay").is_empty()), "Esc ends the peek");

    chord(&PEEK);
    wait(3, || topmost(pid));
    type_text("note");
    wait(3, || windows(pid, "RustDesktopIcons.Search").len() == 1);
    support::point((w.left + 30, w.bottom - 30));
    support::press(true);
    support::press(false);
    let gone = wait(3, || windows(pid, "RustDesktopIcons.Overlay").is_empty() && windows(pid, "RustDesktopIcons.Search").is_empty());
    check(gone, "one click outside ends the peek and closes the search");

    chord(&SEARCH);
    check(wait(3, || windows(pid, "RustDesktopIcons.Search").len() == 1), "the search shortcut opens the search");
    chord(&[VK_ESCAPE]);
    wait(3, || windows(pid, "RustDesktopIcons.Search").is_empty());

    let lv = desk::listview();
    let uncovered = support::free_desktop_spot(&fences(pid)).is_none();
    if uncovered {
        support::show_desktop();
    }
    match support::free_desktop_spot(&fences(pid)) {
        Some(p) => {
            double_click(p);
            check(wait(3, || fences(pid).is_empty()), "double-click on the empty desktop hides the fences");
            check(!unsafe { IsWindowVisible(lv) }.as_bool(), "double-click on the empty desktop hides the icons");
            thread::sleep(Duration::from_millis(600));
            double_click(p);
            check(wait(3, || fences(pid).len() == 2), "a second double-click shows the fences again");
            check(unsafe { IsWindowVisible(lv) }.as_bool(), "a second double-click shows the icons again");
        }
        None => println!("[SKIP] double-click: no empty desktop spot visible"),
    }
    quit(&mut app);
    if uncovered {
        support::show_desktop();
    }
    if !unsafe { IsWindowVisible(lv) }.as_bool() {
        let _ = unsafe { ShowWindow(lv, SW_SHOWNA) };
        check(false, "desktop icons visible after quitting");
    }
    if fails.is_empty() {
        let _ = fs::remove_dir_all(&root);
        println!("all peek checks passed");
    } else {
        println!("{} failed", fails.len());
        std::process::exit(1);
    }
}
