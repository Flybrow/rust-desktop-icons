mod support;

use serde_json::json;
use std::{
    env, fs,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use support::{drag, fences, point, work};
use windows::Win32::Foundation::{LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

fn nearest(rects: &[RECT], (x, y): (i32, i32)) -> RECT {
    *rects.iter().min_by_key(|r| (r.left - x).abs() + (r.top - y).abs()).unwrap_or(&RECT::default())
}

fn settle(pid: u32, at: (i32, i32)) -> RECT {
    let start = fences(pid);
    let t = Instant::now();
    while fences(pid) == start && t.elapsed() < Duration::from_millis(1500) {
        thread::sleep(Duration::from_millis(20));
    }
    let mut last = fences(pid);
    while t.elapsed() < Duration::from_secs(4) {
        thread::sleep(Duration::from_millis(250));
        let now = fences(pid);
        if now == last {
            break;
        }
        last = now;
    }
    let mut best = nearest(&last, at);
    for r in &last {
        let inside = (r.left..r.right).contains(&at.0) && (r.top..r.bottom).contains(&at.1);
        if inside {
            best = *r;
        }
    }
    best
}

fn hover(p: (i32, i32)) {
    for d in 0..4 {
        point((p.0 + d, p.1));
        thread::sleep(Duration::from_millis(30));
    }
}

fn capture(r: &RECT, path: &Path) {
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    let mut px = vec![0u8; (w * h * 4) as usize];
    unsafe {
        let screen = GetDC(None);
        let dc = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(dc, bmp.into());
        let _ = BitBlt(dc, 0, 0, w, h, Some(screen), r.left, r.top, SRCCOPY);
        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            },
            ..Default::default()
        };
        GetDIBits(dc, bmp, 0, h as u32, Some(px.as_mut_ptr() as _), &mut bi, DIB_RGB_COLORS);
        SelectObject(dc, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(dc);
        ReleaseDC(None, screen);
    }
    let size = 54 + px.len() as u32;
    let mut out = vec![];
    out.extend(b"BM");
    out.extend(size.to_le_bytes());
    out.extend([0u8; 4]);
    out.extend(54u32.to_le_bytes());
    out.extend(40u32.to_le_bytes());
    out.extend(w.to_le_bytes());
    out.extend((-h).to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(32u16.to_le_bytes());
    out.extend([0u8; 24]);
    out.extend(px);
    let _ = fs::write(path, out);
}

struct Case {
    name: &'static str,
    at: (i32, i32),
    vertical: bool,
    far: bool,
}

fn main() {
    let shots = env::args().skip_while(|a| a != "--shots").nth(1);
    let w = work();
    let root = env::temp_dir().join(format!("rdi-unroll-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    let cases = [
        Case { name: "horizontal, room below", at: (w.left + 300, w.top + 100), vertical: false, far: false },
        Case { name: "horizontal, screen bottom", at: (w.left + 300, w.bottom - 120), vertical: false, far: true },
        Case { name: "vertical, room on the right", at: (w.left + 60, w.top + 250), vertical: true, far: false },
        Case { name: "vertical, screen right edge", at: (w.right - 60, w.top + 250), vertical: true, far: true },
    ];
    let mut cfg_fences = vec![];
    for (i, c) in cases.iter().enumerate() {
        let id = i as u64 + 1;
        let dir = data.join("fences").join(id.to_string());
        fs::create_dir_all(&dir).unwrap();
        for k in 0..12 {
            fs::write(dir.join(format!("item {k}.txt")), "x").unwrap();
        }
        let tabs = json!([{"id": id, "title": "Unroll"}, {"id": id + 100, "title": "Two"}]);
        fs::create_dir_all(data.join("fences").join((id + 100).to_string())).unwrap();
        cfg_fences
            .push(json!({"id": id, "x": c.at.0, "y": c.at.1, "w": 360, "h": 240, "rolled": true, "vertical": c.vertical, "tabs": tabs}));
    }
    let cfg = json!({"auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "fences": cfg_fences});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let exe = env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("rust-desktop-icons.exe");
    let mut app = Command::new(exe)
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
    let away = (w.left + 5, (w.top + w.bottom) / 2);
    let mut fails = vec![];
    let mut check = |ok: bool, what: String| {
        println!("[{}] {what}", if ok { "PASS" } else { "FAIL" });
        if !ok {
            fails.push(what);
        }
    };
    let t = Instant::now();
    while fences(pid).len() < cases.len() && t.elapsed() < Duration::from_secs(10) {
        thread::sleep(Duration::from_millis(100));
    }
    thread::sleep(Duration::from_millis(800));
    check(fences(pid).len() == cases.len(), format!("{} rolled fences shown", cases.len()));
    for c in &cases {
        let r0 = nearest(&fences(pid), c.at);
        let bar = if c.vertical { r0.right - r0.left } else { r0.bottom - r0.top };
        let grip = if c.vertical { (r0.left + bar / 2, r0.top + 60) } else { (r0.left + 60, r0.top + bar / 2) };
        hover(grip);
        let open = settle(pid, grip);
        println!(
            "      {}: rolled {:?} -> open {:?}",
            c.name,
            (r0.left, r0.top, r0.right, r0.bottom),
            (open.left, open.top, open.right, open.bottom)
        );
        if let Some(dir) = &shots {
            capture(&open, &Path::new(dir).join(format!("{}.bmp", c.name.replace([',', ' '], "_"))));
        }
        let grew = if c.vertical { open.right - open.left > bar * 2 } else { open.bottom - open.top > bar * 2 };
        check(grew, format!("{} unrolls on hover", c.name));
        let way = match (c.vertical, c.far) {
            (false, false) => open.top == r0.top && open.bottom > r0.bottom,
            (false, true) => open.bottom == r0.bottom && open.top < r0.top,
            (true, false) => open.left == r0.left && open.right > r0.right,
            (true, true) => open.right == r0.right && open.left < r0.left,
        };
        check(way, format!("{} unrolls away from the edge", c.name));
        check(
            open.left >= w.left && open.right <= w.right && open.top >= w.top && open.bottom <= w.bottom,
            format!("{} stays on screen", c.name),
        );
        point(away);
        let back = settle(pid, grip);
        check(back == r0, format!("{} rolls back to the same place", c.name));
        if c.far {
            hover(grip);
            let open = settle(pid, grip);
            let bar_at = if c.vertical { (open.right - bar / 2, open.top + 60) } else { (open.left + 60, open.bottom - bar / 2) };
            let shift = if c.vertical { (0, 120) } else { (200, 0) };
            drag(bar_at, (bar_at.0 + shift.0, bar_at.1 + shift.1));
            let moved = settle(pid, (bar_at.0 + shift.0, bar_at.1 + shift.1));
            let anchored = if c.vertical {
                moved.right == open.right && moved.top != open.top
            } else {
                moved.bottom == open.bottom && moved.left != open.left
            };
            check(
                anchored,
                format!("{} can be dragged by its bar while open {:?}", c.name, (moved.left, moved.top, moved.right, moved.bottom)),
            );
            point(away);
            let back = settle(pid, (bar_at.0 + shift.0, bar_at.1 + shift.1));
            let onto = if c.vertical {
                back.right == moved.right && back.right - back.left == bar
            } else {
                back.bottom == moved.bottom && back.bottom - back.top == bar
            };
            check(onto, format!("{} rolls back onto its moved bar", c.name));
        }
    }
    point((saved.x, saved.y));
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
