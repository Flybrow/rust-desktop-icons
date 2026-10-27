#![allow(dead_code)]
use std::{thread, time::Duration};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::BOOL;

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
    let (pid, name, out) = unsafe { &mut *(lp.0 as *mut (u32, &str, Vec<HWND>)) };
    let mut p = 0;
    unsafe { GetWindowThreadProcessId(h, Some(&mut p)) };
    if (*pid == 0 || p == *pid) && class(h) == *name && unsafe { IsWindowVisible(h) }.as_bool() {
        out.push(h);
    }
    true.into()
}

pub fn windows(pid: u32, name: &str) -> Vec<HWND> {
    let mut st = (pid, name, Vec::<HWND>::new());
    let _ = unsafe { EnumWindows(Some(collect), LPARAM(&mut st as *mut _ as isize)) };
    st.2
}

pub fn rect(h: HWND) -> RECT {
    let mut r = RECT::default();
    let _ = unsafe { GetWindowRect(h, &mut r) };
    r
}

pub fn fences(pid: u32) -> Vec<RECT> {
    windows(pid, "RustDesktopIcons.Fence").into_iter().map(rect).collect()
}

pub fn point(p: (i32, i32)) {
    unsafe { SetCursorPos(p.0, p.1).unwrap() };
}

pub fn cursor() -> (i32, i32) {
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p).unwrap() };
    (p.x, p.y)
}

fn button(flag: MOUSE_EVENT_FLAGS) {
    let input = INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { dwFlags: flag, ..Default::default() } } };
    unsafe { SendInput(&[input], size_of::<INPUT>() as i32) };
    thread::sleep(Duration::from_millis(60));
}

pub fn drag(from: (i32, i32), to: (i32, i32)) {
    point(from);
    thread::sleep(Duration::from_millis(150));
    button(MOUSEEVENTF_LEFTDOWN);
    for k in 1..=20 {
        point((from.0 + (to.0 - from.0) * k / 20, from.1 + (to.1 - from.1) * k / 20));
        thread::sleep(Duration::from_millis(30));
    }
    thread::sleep(Duration::from_millis(200));
    button(MOUSEEVENTF_LEFTUP);
}

pub fn on_desktop((x, y): (i32, i32)) -> bool {
    let root = unsafe { GetAncestor(WindowFromPoint(POINT { x, y }), GA_ROOT) };
    matches!(class(root).as_str(), "Progman" | "WorkerW")
}

pub fn free_desktop_spot(avoid: &[RECT]) -> Option<(i32, i32)> {
    let w = work();
    let inside = |r: &RECT, (x, y): (i32, i32)| x > r.left - 150 && x < r.right + 150 && y > r.top - 150 && y < r.bottom + 150;
    (0..40)
        .flat_map(|i| {
            (0..40).map(move |j| (w.left + 60 + i * (w.right - w.left - 120) / 39, w.top + 60 + j * (w.bottom - w.top - 120) / 39))
        })
        .find(|&p| !avoid.iter().any(|r| inside(r, p)) && on_desktop(p))
}
