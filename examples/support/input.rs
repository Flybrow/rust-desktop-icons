use std::{thread, time::Duration};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

fn send(inputs: &[INPUT]) {
    unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
    thread::sleep(Duration::from_millis(40));
}

fn key(vk: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: scan, dwFlags: flags, ..Default::default() } } }
}

pub fn chord(keys: &[VIRTUAL_KEY]) {
    let down: Vec<INPUT> = keys.iter().map(|k| key(*k, 0, KEYBD_EVENT_FLAGS(0))).collect();
    let up: Vec<INPUT> = keys.iter().rev().map(|k| key(*k, 0, KEYEVENTF_KEYUP)).collect();
    send(&down);
    send(&up);
    thread::sleep(Duration::from_millis(150));
}

pub fn type_text(s: &str) {
    for c in s.encode_utf16() {
        send(&[key(VIRTUAL_KEY(0), c, KEYEVENTF_UNICODE), key(VIRTUAL_KEY(0), c, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP)]);
    }
    thread::sleep(Duration::from_millis(150));
}

pub fn double_click(p: (i32, i32)) {
    point(p);
    thread::sleep(Duration::from_millis(150));
    for down in [true, false, true, false] {
        press(down);
    }
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

pub fn press(down: bool) {
    button(if down { MOUSEEVENTF_LEFTDOWN } else { MOUSEEVENTF_LEFTUP });
}

pub fn right(down: bool) {
    button(if down { MOUSEEVENTF_RIGHTDOWN } else { MOUSEEVENTF_RIGHTUP });
}

fn shove((x, y): (i32, i32)) {
    let (vx, vy, vw, vh) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    let norm = |v: i32, o: i32, len: i32| (((v - o) as i64 * 65535 + (len as i64 - 1) / 2) / (len as i64 - 1).max(1)) as i32;
    let mi = MOUSEINPUT {
        dx: norm(x, vx, vw),
        dy: norm(y, vy, vh),
        dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
        ..Default::default()
    };
    let input = INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi } };
    unsafe { SendInput(&[input], size_of::<INPUT>() as i32) };
    if cursor() != (x, y) {
        point((x, y));
    }
}

pub fn glide(from: (i32, i32), to: (i32, i32)) {
    for k in 1..=20 {
        shove((from.0 + (to.0 - from.0) * k / 20, from.1 + (to.1 - from.1) * k / 20));
        thread::sleep(Duration::from_millis(30));
    }
    thread::sleep(Duration::from_millis(200));
}

pub fn drag(from: (i32, i32), to: (i32, i32)) {
    point(from);
    thread::sleep(Duration::from_millis(150));
    press(true);
    glide(from, to);
    press(false);
}

pub fn show_desktop() {
    chord(&[VK_LWIN, VK_D]);
    thread::sleep(Duration::from_millis(800));
}
