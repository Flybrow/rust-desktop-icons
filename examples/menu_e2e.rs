mod support;

use serde_json::json;
use std::{env, fs, thread, time::Duration};
use support::{app, chord, fences, quit, right_click, wait, windows, work};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

fn blocker(r: RECT) {
    thread::spawn(move || unsafe {
        let style = WS_POPUP | WS_VISIBLE | WS_BORDER;
        let h = CreateWindowExW(
            WS_EX_TOPMOST,
            w!("STATIC"),
            w!("blocker"),
            style,
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let _ = SetForegroundWindow(h);
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }
    });
}

fn on_top(h: HWND) -> bool {
    let r = support::rect(h);
    let c = POINT { x: (r.left + r.right) / 2, y: (r.top + r.bottom) / 2 };
    (unsafe { GetAncestor(WindowFromPoint(c), GA_ROOT) }) == h
}

fn main() {
    let w = work();
    let root = env::temp_dir().join(format!("rdi-menu-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    fs::create_dir_all(data.join("fences").join("1")).unwrap();
    fs::write(data.join("fences").join("1").join("note.txt"), "x").unwrap();
    let cfg = json!({"auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"),
        "fences": [{"id": 1, "x": w.left + 100, "y": w.top + 100, "w": 300, "h": 300, "auto_height": false, "tabs": [{"id": 1, "title": "Menu"}]}]});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let mut app = app(&root, &[], &[]);
    let pid = app.id();
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| {
        support::verdict(ok, what, &mut fails);
    };
    check(wait(10, || fences(pid).len() == 1), "fence shown");
    let f = fences(pid)[0];
    blocker(RECT { left: f.right + 2, top: w.top, right: w.right, bottom: w.bottom });
    thread::sleep(Duration::from_millis(600));
    right_click((f.right - 20, f.top + 200));
    check(wait(3, || windows(0, "#32768").len() == 1), "the fence menu opens");
    let menu = windows(0, "#32768")[0];
    let hmenu = HMENU(unsafe { SendMessageW(menu, MN_GETHMENU, None, None) }.0 as _);
    let sort = (0..unsafe { GetMenuItemCount(Some(hmenu)) }).find(|&i| !unsafe { GetSubMenu(hmenu, i) }.is_invalid()).unwrap_or(0);
    let mut r = RECT::default();
    let _ = unsafe { GetMenuItemRect(None, hmenu, sort as u32, &mut r) };
    let at = ((r.left + r.right) / 2, (r.top + r.bottom) / 2);
    support::glide((at.0 - 30, at.1), at);
    thread::sleep(Duration::from_millis(900));
    let menus = windows(0, "#32768");
    check(menus.len() >= 2, &format!("a submenu opens ({} menus)", menus.len()));
    for m in menus.iter().filter(|m| !on_top(**m)) {
        let r = support::rect(*m);
        let c = POINT { x: (r.left + r.right) / 2, y: (r.top + r.bottom) / 2 };
        let over = unsafe { GetAncestor(WindowFromPoint(c), GA_ROOT) };
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(over, Some(&mut pid)) };
        let mut buf = [0u16; 64];
        let n = unsafe { GetClassNameW(over, &mut buf) } as usize;
        println!(
            "      menu {:?} covered by {} (pid {pid}, app {})",
            (r.left, r.top, r.right, r.bottom),
            String::from_utf16_lossy(&buf[..n]),
            app.id()
        );
    }
    check(menus.iter().all(|m| on_top(*m)), "every menu is drawn above the other windows");
    if let Some(dir) = env::args().skip_while(|a| a != "--shots").nth(1) {
        let r = menus.iter().map(|m| support::rect(*m)).fold(f, |a, b| RECT {
            left: a.left.min(b.left),
            top: a.top.min(b.top),
            right: a.right.max(b.right),
            bottom: a.bottom.max(b.bottom),
        });
        support::capture(&r, &std::path::Path::new(&dir).join("menu.bmp"));
    }
    chord(&[VK_ESCAPE]);
    chord(&[VK_ESCAPE]);
    wait(3, || windows(0, "#32768").is_empty());
    right_click((f.left + 60, f.top + 80));
    let shell = wait(3, || windows(0, "#32768").len() == 1).then(|| windows(0, "#32768")[0]);
    let entries = shell.map_or(0, |m| unsafe { GetMenuItemCount(Some(HMENU(SendMessageW(m, MN_GETHMENU, None, None).0 as _))) });
    check(entries > 3, &format!("the Windows menu of an item opens ({entries} entries)"));
    check(shell.is_some_and(on_top), "the Windows menu of an item is drawn above the other windows");
    chord(&[VK_ESCAPE]);
    quit(&mut app);
    if fails.is_empty() {
        let _ = fs::remove_dir_all(&root);
        println!("all menu checks passed");
    } else {
        println!("{} failed", fails.len());
        std::process::exit(1);
    }
}
