#![allow(dead_code, unused_imports)]
pub mod desk;
mod input;

pub use input::{chord, cursor, double_click, drag, glide, point, press, right, show_desktop, type_text};

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

fn class(h: HWND) -> String {
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

pub fn quit(app: &mut Child) {
    for h in find(app.id(), "RustDesktopIcons.Tray.dev", true) {
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

pub fn fences(pid: u32) -> Vec<RECT> {
    windows(pid, "RustDesktopIcons.Fence").into_iter().map(rect).collect()
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
