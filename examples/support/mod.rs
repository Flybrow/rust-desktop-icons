#![allow(dead_code, unused_imports)]
pub mod clip;
pub mod desk;
mod input;
pub mod menu;
pub mod usage;

pub use input::{
    chord, click, ctrl_wheel, cursor, double_click, drag, glide, hold, point, press, right, right_click, show_desktop, type_text, wheel,
};

use std::{
    fs,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::BOOL;

pub fn app(root: &Path, env: &[(&str, &str)], args: &[&str]) -> Child {
    let exe = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("rust-desktop-icons.exe");
    let mut c = Command::new(exe);
    c.args(args).env("LOCALAPPDATA", root).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    for (k, v) in env {
        c.env(k, v);
    }
    c.spawn().expect("build the app first: cargo build")
}

pub fn waited(secs: u64, mut f: impl FnMut() -> bool) -> Option<u128> {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(secs) {
        if f() {
            return Some(t.elapsed().as_millis());
        }
        thread::sleep(Duration::from_millis(10));
    }
    None
}

pub fn wait(secs: u64, f: impl FnMut() -> bool) -> bool {
    waited(secs, f).is_some()
}

pub fn poll<T>(secs: u64, mut f: impl FnMut() -> Option<T>) -> Option<T> {
    let mut out = None;
    waited(secs, || {
        out = f();
        out.is_some()
    });
    out
}

pub fn work() -> RECT {
    let mut r = RECT::default();
    unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut r as *mut _ as _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)).unwrap() };
    r
}

pub fn class(h: HWND) -> String {
    let mut buf = [0u16; 64];
    let n = unsafe { GetClassNameW(h, &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..n])
}

unsafe extern "system" fn collect(h: HWND, lp: LPARAM) -> BOOL {
    let (pid, name, all, out) = unsafe { &mut *(lp.0 as *mut (u32, &str, bool, Vec<HWND>)) };
    let mut p = 0;
    unsafe { GetWindowThreadProcessId(h, Some(&mut p)) };
    if (*pid == 0 || p == *pid) && class(h) == *name && (*all || unsafe { IsWindowVisible(h) }.as_bool()) {
        out.push(h);
    }
    true.into()
}

fn find(pid: u32, name: &str, all: bool) -> Vec<HWND> {
    let mut st = (pid, name, all, Vec::<HWND>::new());
    let _ = unsafe { EnumWindows(Some(collect), LPARAM(&mut st as *mut _ as isize)) };
    st.3
}

pub fn windows(pid: u32, name: &str) -> Vec<HWND> {
    find(pid, name, false)
}

pub fn windows_all(pid: u32, name: &str) -> Vec<HWND> {
    find(pid, name, true)
}

pub fn quit(app: &mut Child) {
    for h in windows_all(app.id(), "RustDesktopIcons.Tray.dev") {
        let _ = unsafe { PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0)) };
    }
    for _ in 0..50 {
        if app.try_wait().ok().flatten().is_some() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let _ = app.kill();
    let _ = app.wait();
}

pub fn rect(h: HWND) -> RECT {
    let mut r = RECT::default();
    let _ = unsafe { GetWindowRect(h, &mut r) };
    r
}

pub fn drawn(h: HWND) -> bool {
    !unsafe { GetPropW(h, windows::core::w!("RDI_DRAWN")) }.is_invalid()
}

pub fn fences(pid: u32) -> Vec<RECT> {
    windows(pid, "RustDesktopIcons.Fence").into_iter().filter(|h| drawn(*h)).map(rect).collect()
}

pub fn on_desktop((x, y): (i32, i32)) -> bool {
    let root = unsafe { GetAncestor(WindowFromPoint(POINT { x, y }), GA_ROOT) };
    matches!(class(root).as_str(), "Progman" | "WorkerW")
}

pub fn free_desktop_spot(avoid: &[RECT]) -> Option<(i32, i32)> {
    let icons: Vec<(i32, i32)> = desk::positions().into_iter().map(|(_, p)| desk::screen(p)).collect();
    let w = work();
    let inside = |r: &RECT, (x, y): (i32, i32)| x > r.left - 150 && x < r.right + 150 && y > r.top - 150 && y < r.bottom + 150;
    let on_icon = |p: &(i32, i32)| icons.iter().any(|q| (q.0 - 20..q.0 + 100).contains(&p.0) && (q.1 - 20..q.1 + 100).contains(&p.1));
    let spots: Vec<(i32, i32)> = (0..40)
        .flat_map(|i| {
            (0..40).map(move |j| (w.left + 60 + i * (w.right - w.left - 120) / 39, w.top + 60 + j * (w.bottom - w.top - 120) / 39))
        })
        .filter(|&p| !avoid.iter().any(|r| inside(r, p)) && on_desktop(p))
        .collect();
    spots.iter().find(|p| !on_icon(p)).or(spots.first()).copied()
}

pub fn capture(r: &RECT, path: &Path) {
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

pub fn title(h: HWND) -> String {
    let mut b = [0u16; 512];
    let n = unsafe { GetWindowTextW(h, &mut b) } as usize;
    String::from_utf16_lossy(&b[..n])
}

pub fn explorer(name: &str) -> Option<HWND> {
    let ours = |t: String| {
        [" - ", " : ", " — "].iter().fold(t.as_str(), |t, sep| t.split(sep).next().unwrap_or(t)).rsplit('\\').next() == Some(name)
    };
    windows(0, "CabinetWClass").into_iter().find(|h| ours(title(*h)))
}

pub fn close(h: HWND) {
    let _ = unsafe { PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0)) };
}

pub fn saved(data: &Path) -> serde_json::Value {
    fs::read(data.join("config.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

pub fn fence_cfg(data: &Path, id: u64) -> serde_json::Value {
    saved(data)["fences"].as_array().and_then(|a| a.iter().find(|f| f["id"] == id).cloned()).unwrap_or_default()
}

pub fn answer(pid: u32, keys: &[windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY]) -> bool {
    let shown = wait(3, || !windows(pid, "#32770").is_empty());
    if let Some(d) = windows(pid, "#32770").first() {
        let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(*d) };
        thread::sleep(Duration::from_millis(200));
        chord(keys);
    }
    shown
}

thread_local! {
    static SHOT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

pub fn verdict(ok: bool, what: &str, fails: &mut Vec<String>) {
    println!("[{}] {what}", if ok { "PASS" } else { "FAIL" });
    if !ok {
        fails.push(what.to_string());
    }
    SHOT.with(|n| n.set(n.get() + 1));
    if let Ok(dir) = std::env::var("SHOTS") {
        let exe = std::env::current_exe().ok().and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned())).unwrap_or_default();
        let slug: String = what.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).take(60).collect();
        let screen = unsafe {
            RECT {
                left: GetSystemMetrics(SM_XVIRTUALSCREEN),
                top: GetSystemMetrics(SM_YVIRTUALSCREEN),
                right: GetSystemMetrics(SM_XVIRTUALSCREEN) + GetSystemMetrics(SM_CXVIRTUALSCREEN),
                bottom: GetSystemMetrics(SM_YVIRTUALSCREEN) + GetSystemMetrics(SM_CYVIRTUALSCREEN),
            }
        };
        let _ = fs::create_dir_all(&dir);
        let (n, verdict) = (SHOT.with(|n| n.get()), if ok { "PASS" } else { "FAIL" });
        capture(&screen, &Path::new(&dir).join(format!("{exe}-{n:03}-{verdict}-{slug}.bmp")));
    }
}
