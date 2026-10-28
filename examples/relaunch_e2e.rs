use std::{
    env, fs,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

fn tray() -> Option<(HWND, u32)> {
    let h = unsafe { FindWindowW(w!("RustDesktopIcons.Tray.dev"), None) }.ok()?;
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
    Some((h, pid))
}

fn wait(secs: u64, mut f: impl FnMut() -> bool) -> bool {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(secs) {
        if f() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

fn start(root: &Path, args: &[&str]) -> Child {
    let exe = env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("rust-desktop-icons.exe");
    Command::new(exe).args(args).env("LOCALAPPDATA", root).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap()
}

fn exited(c: &mut Child) -> bool {
    c.try_wait().ok().flatten().is_some()
}

fn main() {
    let root = env::temp_dir().join(format!("rdi-relaunch-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("config.json"), r#"{"auto_update": false, "auto_sort": false, "version": "0.0.0"}"#).unwrap();
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| {
        println!("[{}] {what}", if ok { "PASS" } else { "FAIL" });
        if !ok {
            fails.push(what.to_string());
        }
    };
    let mut old = start(&root, &[]);
    check(wait(10, || tray().is_some_and(|(_, p)| p == old.id())), "first instance running");
    let mut new = start(&root, &["--relaunch"]);
    thread::sleep(Duration::from_millis(1500));
    check(!exited(&mut new) && tray().is_some_and(|(_, p)| p == old.id()), "relaunched instance waits while the old one is still running");
    if let Some((h, _)) = tray() {
        let _ = unsafe { PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0)) };
    }
    check(wait(10, || exited(&mut old)), "old instance closes on request");
    check(wait(10, || tray().is_some_and(|(_, p)| p == new.id())), "relaunched instance takes over once the old one is gone");
    let mut extra = start(&root, &["--relaunch"]);
    check(wait(25, || exited(&mut extra)), "a second relaunch exits quietly when an instance is already running");
    let ended = tray().map(|(h, _)| unsafe {
        let mut r = 0usize;
        let _ = SendMessageTimeoutW(h, WM_QUERYENDSESSION, WPARAM(0), LPARAM(0), SMTO_ABORTIFHUNG, 3000, Some(&mut r));
        let _ = SendMessageTimeoutW(h, WM_ENDSESSION, WPARAM(1), LPARAM(0), SMTO_ABORTIFHUNG, 3000, None);
        r
    });
    check(ended == Some(1), "agrees to end the session");
    check(wait(5, || exited(&mut new)), "exits right after Windows ends the session");
    let log = fs::read_to_string(data.join("errors.log")).unwrap_or_default();
    check(
        log.contains("started after an update") && log.contains("closed by Windows") && log.contains("closed on request"),
        "start and close reasons are logged",
    );
    for c in [&mut old, &mut new, &mut extra] {
        let _ = c.kill();
        let _ = c.wait();
    }
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("RELAUNCH E2E OK");
    } else {
        println!("RELAUNCH E2E FAILED: {}", fails.join("; "));
        std::process::exit(1);
    }
}
