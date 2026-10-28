mod support;

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_CLOSE};
use windows::core::w;

fn wait<T>(secs: u64, mut f: impl FnMut() -> Option<T>) -> Option<T> {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(secs) {
        if let Some(v) = f() {
            return Some(v);
        }
        thread::sleep(Duration::from_millis(100));
    }
    None
}

fn dialog(pid: u32) -> Option<HWND> {
    support::windows(pid, "RustDesktopIcons.Dialog").into_iter().next()
}

fn button(r: &RECT, k: i32) -> (i32, i32) {
    (r.right - 82 * k / 96, r.bottom - 45 * k / 96)
}

fn click(p: (i32, i32)) {
    let saved = support::cursor();
    support::drag(p, p);
    support::point(saved);
}

fn start(root: &Path, fake: bool) -> Child {
    let exe = env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("rust-desktop-icons.exe");
    let mut c = Command::new(exe);
    c.env("LOCALAPPDATA", root).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    if fake {
        c.env("RDI_FAKE_VERSION", "0.0.1");
    }
    c.spawn().unwrap()
}

fn stop(mut app: Child) {
    if let Ok(t) = unsafe { FindWindowW(w!("RustDesktopIcons.Tray.dev"), None) } {
        let _ = unsafe { PostMessageW(Some(t), WM_CLOSE, WPARAM(0), LPARAM(0)) };
    }
    thread::sleep(Duration::from_secs(1));
    let _ = app.kill();
    let _ = app.wait();
}

fn main() {
    let shots: Option<PathBuf> = env::args().skip_while(|a| a != "--shots").nth(1).map(PathBuf::from);
    let shot = |r: &RECT, name: &str| {
        if let Some(d) = &shots {
            support::capture(r, &d.join(format!("{name}.bmp")));
        }
    };
    let k = unsafe { windows::Win32::UI::HiDpi::GetDpiForSystem() } as i32;
    let root = env::temp_dir().join(format!("rdi-update-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    fs::create_dir_all(&data).unwrap();
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| {
        println!("[{}] {what}", if ok { "PASS" } else { "FAIL" });
        if !ok {
            fails.push(what.to_string());
        }
    };

    fs::write(data.join("config.json"), r#"{"auto_update": false, "auto_sort": false, "version": "0.0.1"}"#).unwrap();
    let app = start(&root, false);
    let pid = app.id();
    let news = wait(10, || dialog(pid));
    check(news.is_some(), "what's new popup shows after an update");
    if let Some(h) = news {
        thread::sleep(Duration::from_millis(500));
        let r = support::rect(h);
        shot(&r, "whatsnew");
        click(button(&r, k));
        check(wait(5, || dialog(pid).is_none().then_some(())).is_some(), "its OK button closes it");
    }
    stop(app);

    let cfg = fs::read_to_string(data.join("config.json")).unwrap().replace("\"auto_update\": false", "\"auto_update\": true");
    fs::write(data.join("config.json"), cfg).unwrap();
    let app = start(&root, true);
    let pid = app.id();
    let offer = wait(40, || dialog(pid));
    check(offer.is_some(), "automatic check offers the newer release in a popup");
    if let Some(h) = offer {
        thread::sleep(Duration::from_millis(500));
        let r = support::rect(h);
        shot(&r, "available");
        click(button(&r, k));
        let mid = wait(30, || {
            let r = support::rect(h);
            (r.bottom - r.top != 0).then_some(r)
        });
        thread::sleep(Duration::from_millis(400));
        if let Some(r) = mid {
            shot(&support::rect(h), "progress");
            let _ = r;
        }
        let log = data.join("errors.log");
        let done = wait(90, || fs::read_to_string(&log).ok().filter(|l| l.contains("debug build: not installing")));
        check(done.is_some(), "Install downloads and verifies the release, then starts installing");
        thread::sleep(Duration::from_millis(500));
        check(dialog(pid).is_some(), "the popup stays open with the installing progress");
        shot(&support::rect(h), "installing");
    }
    stop(app);
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("UPDATE E2E OK");
    } else {
        println!("UPDATE E2E FAILED: {}", fails.join("; "));
        std::process::exit(1);
    }
}
