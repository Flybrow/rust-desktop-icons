mod support;

use serde_json::json;
use std::{
    env, fs,
    time::{Duration, Instant},
};
use support::{
    answer, app, chord, drag, fences, menu, point, quit, rect, right_click, type_text, usage::usage, wait, windows, windows_all, work,
};
use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{VK_CONTROL, VK_ESCAPE, VK_F, VK_MENU, VK_RETURN, VK_SPACE},
        WindowsAndMessaging::{PostMessageW, WM_APP, WM_LBUTTONDBLCLK, WM_RBUTTONUP},
    },
};

const WM_TRAY: u32 = WM_APP + 1;

fn tray(pid: u32, event: u32) {
    for h in windows_all(pid, "RustDesktopIcons.Tray.dev") {
        let _ = unsafe { PostMessageW(Some(h), WM_TRAY, WPARAM(0), LPARAM(event as isize)) };
    }
    std::thread::sleep(Duration::from_millis(400));
}

fn pause(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

fn main() {
    let minutes: u64 = env::var("SOAK_MIN").ok().and_then(|v| v.parse().ok()).unwrap_or(3);
    let w = work();
    let root = env::temp_dir().join(format!("rdi-soak-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    let dir = |id: u64| data.join("fences").join(id.to_string());
    for id in [1, 2] {
        fs::create_dir_all(dir(id)).unwrap();
    }
    for i in 0..12 {
        fs::write(dir(2).join(format!("roll {i}.txt")), "x").unwrap();
    }
    let (x0, y0) = (w.left + 120, w.top + 80);
    let cfg = json!({"lang": "en", "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "peek_key": {"mods": 3, "vk": 32},
        "search_key": {"mods": 3, "vk": 70}, "fences": [
        {"id": 1, "x": x0, "y": y0, "w": 420, "h": 300, "auto_height": false, "tabs": [{"id": 1, "title": "Busy"}]},
        {"id": 2, "x": x0 + 500, "y": y0, "w": 360, "h": 260, "rolled": true, "tabs": [{"id": 2, "title": "Roll"}]}]});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let mut child = app(&root, &[("RDI_TOP", "1")], &[]);
    let pid = child.id();
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| support::verdict(ok, what, &mut fails);
    check(wait(10, || fences(pid).len() == 2), "two fences shown");
    let away = || point((w.right - 20, w.bottom - 20));
    let end = Instant::now() + Duration::from_secs(minutes * 60);
    let mut samples = vec![];
    let mut rounds = 0;
    while Instant::now() < end && child.try_wait().ok().flatten().is_none() {
        rounds += 1;
        for i in 0..30 {
            let _ = fs::write(dir(1).join(format!("soak {i}.txt")), "x");
        }
        pause(1200);
        for i in 0..30 {
            let _ = fs::remove_file(dir(1).join(format!("soak {i}.txt")));
        }
        pause(800);
        point((x0 + 680, y0 + 12));
        pause(900);
        away();
        pause(900);
        chord(&[VK_CONTROL, VK_MENU, VK_SPACE]);
        pause(600);
        chord(&[VK_ESCAPE]);
        pause(400);
        chord(&[VK_CONTROL, VK_MENU, VK_F]);
        pause(500);
        type_text("soak");
        pause(300);
        chord(&[VK_ESCAPE]);
        pause(400);
        tray(pid, WM_LBUTTONDBLCLK);
        if wait(3, || !windows(pid, "RustDesktopIcons.Settings").is_empty()) {
            windows(pid, "RustDesktopIcons.Settings").into_iter().for_each(support::close);
        }
        pause(500);
        let n = fences(pid).len();
        tray(pid, WM_RBUTTONUP);
        if menu::open() && menu::pick(&["New fence"]) && wait(3, || fences(pid).len() == n + 1) {
            away();
            pause(500);
            let known = [x0, x0 + 500];
            if let Some(h) =
                windows(pid, "RustDesktopIcons.Fence").into_iter().find(|h| !known.iter().any(|x| (rect(*h).left - x).abs() < 30))
            {
                let r = rect(h);
                right_click((r.right - 30, r.bottom - 30));
                if menu::open() && menu::pick(&["Delete fence"]) {
                    answer(pid, &[VK_RETURN]);
                }
            }
            wait(3, || fences(pid).len() == n);
        }
        drag((x0 + 60, y0 + 12), (x0 + 160, y0 + 112));
        pause(400);
        drag((x0 + 160, y0 + 112), (x0 + 60, y0 + 12));
        away();
        pause(1500);
        if let Some(u) = usage(pid) {
            println!(
                "      round {rounds:3}: GDI {:4}  USER {:4}  handles {:5}  private {:6.1} MB",
                u.gdi, u.user, u.handles, u.private_mb
            );
            samples.push(u);
        }
    }
    let alive = child.try_wait().ok().flatten().is_none();
    check(alive && rounds >= 3, &format!("the app survives {rounds} rounds of everyday actions ({minutes} min)"));
    if let (Some(base), Some(last)) = (samples.get(1).copied(), samples.last().copied()) {
        check(last.gdi <= base.gdi + 30, &format!("no GDI leak ({} -> {})", base.gdi, last.gdi));
        check(last.user <= base.user + 30, &format!("no USER object leak ({} -> {})", base.user, last.user));
        check(last.handles <= base.handles + 150, &format!("no handle leak ({} -> {})", base.handles, last.handles));
        check(
            last.private_mb <= base.private_mb + 25.0,
            &format!("memory stays flat ({:.1} -> {:.1} MB)", base.private_mb, last.private_mb),
        );
    } else {
        check(false, "resource usage could be read");
    }
    check(!data.join("crash.log").exists(), "no crash logged");
    quit(&mut child);
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("SOAK E2E OK");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
