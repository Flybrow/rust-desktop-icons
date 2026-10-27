mod support;

use serde_json::{Value, json};
use std::{
    env, fs,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_CLOSE};
use windows::core::w;

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

fn main() {
    let shot = env::args().skip_while(|a| a != "--shots").nth(1);
    let root = env::temp_dir().join(format!("rdi-system-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    fs::create_dir_all(data.join("fences").join("1")).unwrap();
    let all = ["ThisPc", "UserFiles", "Network", "ControlPanel"];
    let cfg = json!({
        "auto_update": false, "auto_sort": false, "version": "0.0.0",
        "fences": [{"id": 1, "x": 200, "y": 200, "w": 440, "h": 300, "tabs": [{"id": 1, "title": "System", "recycle": true, "system": all}]}]
    });
    let file = data.join("config.json");
    fs::write(&file, serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let config = || fs::read(&file).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).unwrap_or(Value::Null);
    let system = || config()["fences"][0]["tabs"][0]["system"].as_array().cloned().unwrap_or_default();
    let exe = env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("rust-desktop-icons.exe");
    let mut app = Command::new(exe)
        .env("LOCALAPPDATA", &root)
        .env("RDI_TOP", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| {
        println!("[{}] {what}", if ok { "PASS" } else { "FAIL" });
        if !ok {
            fails.push(what.to_string());
        }
    };
    check(wait(10, || !support::fences(app.id()).is_empty()), "fence shown");
    let migrated = wait(10, || system().len() == 5 && system()[0] == "RecycleBin" && !config().to_string().contains("\"recycle\""));
    check(migrated, "legacy Recycle Bin setting migrated next to the other system icons");
    thread::sleep(Duration::from_millis(1500));
    let f = support::fences(app.id()).first().copied().unwrap_or_default();
    if let Some(dir) = &shot {
        support::capture(&f, &std::path::Path::new(dir).join("system.bmp"));
    }
    let k = unsafe { windows::Win32::UI::HiDpi::GetDpiForSystem() } as i32;
    let (cell, cols) = (104 * k / 96, ((f.right - f.left) / (104 * k / 96)).max(1));
    let item = (f.left + ((f.right - f.left) - cols * cell) / 2 + cell / 2, f.top + (34 + 4) * k / 96 + cell / 2);
    match support::free_desktop_spot(&[f]) {
        Some(spot) => {
            let saved = support::cursor();
            support::drag(item, spot);
            support::point(saved);
            check(
                wait(5, || system().len() == 4 && !system().iter().any(|s| s == "RecycleBin")),
                "dragging a system icon onto the desktop takes it out of the fence",
            );
        }
        None => println!("[SKIP] drag a system icon onto the desktop: no free desktop area visible"),
    }
    if let Ok(t) = unsafe { FindWindowW(w!("RustDesktopIcons.Tray.dev"), None) } {
        let _ = unsafe { PostMessageW(Some(t), WM_CLOSE, WPARAM(0), LPARAM(0)) };
    }
    thread::sleep(Duration::from_secs(1));
    let _ = app.kill();
    let _ = app.wait();
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("SYSTEM E2E OK");
    } else {
        println!("SYSTEM E2E FAILED: {}", fails.join("; "));
        std::process::exit(1);
    }
}
