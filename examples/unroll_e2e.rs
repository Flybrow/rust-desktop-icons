use serde_json::json;
use std::{
    env, fs,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{BOOL, w};

fn work() -> RECT {
    let mut r = RECT::default();
    unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut r as *mut _ as _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)).unwrap() };
    r
}

unsafe extern "system" fn collect(h: HWND, lp: LPARAM) -> BOOL {
    let (pid, out) = unsafe { &mut *(lp.0 as *mut (u32, Vec<HWND>)) };
    let mut p = 0;
    unsafe { GetWindowThreadProcessId(h, Some(&mut p)) };
    let mut class = [0u16; 64];
    let n = unsafe { GetClassNameW(h, &mut class) } as usize;
    if p == *pid && String::from_utf16_lossy(&class[..n]) == "RustDesktopIcons.Fence" {
        out.push(h);
    }
    true.into()
}

fn fences(pid: u32) -> Vec<RECT> {
    let mut st = (pid, Vec::<HWND>::new());
    let _ = unsafe { EnumWindows(Some(collect), LPARAM(&mut st as *mut _ as isize)) };
    let mut rects: Vec<RECT> =
        st.1.iter()
            .map(|h| {
                let mut r = RECT::default();
                let _ = unsafe { GetWindowRect(*h, &mut r) };
                r
            })
            .collect();
    rects.sort_by_key(|r| r.top);
    rects
}

fn settle(pid: u32) -> Vec<RECT> {
    let mut last = fences(pid);
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(3) {
        thread::sleep(Duration::from_millis(250));
        let now = fences(pid);
        if now == last {
            break;
        }
        last = now;
    }
    last
}

fn hover(r: &RECT) {
    let (x, y) = ((r.left + r.right) / 2, r.top + 8);
    for d in 0..4 {
        unsafe { SetCursorPos(x + d, y).unwrap() };
        thread::sleep(Duration::from_millis(30));
    }
}

fn main() {
    let w = work();
    let root = env::temp_dir().join(format!("rdi-unroll-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    for id in [1, 2] {
        let dir = data.join("fences").join(id.to_string());
        fs::create_dir_all(&dir).unwrap();
        for i in 0..12 {
            fs::write(dir.join(format!("item {i}.txt")), "x").unwrap();
        }
    }
    let low = w.bottom - 120;
    let fence = |id: u64, y: i32| json!({"id": id, "x": w.left + 300, "y": y, "w": 360, "h": 240, "rolled": true, "tabs": [{"id": id, "title": "Unroll"}]});
    let cfg = json!({"auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "fences": [fence(1, w.top + 100), fence(2, low)]});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let exe = env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("rust-desktop-icons.exe");
    let mut app: Child = Command::new(exe)
        .env("LOCALAPPDATA", &root)
        .env("RDI_TOP", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let pid = app.id();
    let mut saved = POINT::default();
    unsafe { GetCursorPos(&mut saved).unwrap() };
    let mut fails = vec![];
    let mut check = |ok: bool, what: String| {
        println!("[{}] {what}", if ok { "PASS" } else { "FAIL" });
        if !ok {
            fails.push(what);
        }
    };
    let t = Instant::now();
    while fences(pid).len() < 2 && t.elapsed() < Duration::from_secs(10) {
        thread::sleep(Duration::from_millis(100));
    }
    let rolled = settle(pid);
    check(rolled.len() == 2, format!("two rolled fences shown: {rolled:?}"));
    if rolled.len() == 2 {
        for (i, name) in [(0, "top"), (1, "bottom")] {
            let r0 = rolled[i];
            hover(&r0);
            let open = settle(pid)[i];
            println!("      {name}: rolled {:?} -> open {:?}", (r0.top, r0.bottom), (open.top, open.bottom));
            check(open.bottom - open.top > (r0.bottom - r0.top) * 2, format!("{name} fence unrolls on hover"));
            if i == 0 {
                check(open.top == r0.top && open.bottom > r0.bottom, "fence with room below unrolls downward".into());
            } else {
                check(open.bottom == r0.bottom && open.top < r0.top, "fence without room below unrolls upward".into());
                check(open.bottom <= w.bottom, "upward fence stays inside the work area".into());
            }
            unsafe { SetCursorPos(w.left + 5, (w.top + w.bottom) / 2).unwrap() };
            let back = settle(pid)[i];
            check(back == r0, format!("{name} fence rolls back to the same place {:?}", (back.top, back.bottom)));
        }
    }
    unsafe { SetCursorPos(saved.x, saved.y).unwrap() };
    if let Ok(t) = unsafe { FindWindowW(w!("RustDesktopIcons.Tray.dev"), None) } {
        let _ = unsafe { PostMessageW(Some(t), WM_CLOSE, WPARAM(0), LPARAM(0)) };
    }
    thread::sleep(Duration::from_secs(1));
    let _ = app.kill();
    let _ = app.wait();
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("UNROLL E2E OK");
    } else {
        println!("UNROLL E2E FAILED: {}", fails.join("; "));
        std::process::exit(1);
    }
}
