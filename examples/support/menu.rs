use super::{glide, poll, press, windows};
use std::{thread, time::Duration};
use windows::Win32::{
    Foundation::{LPARAM, RECT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{VK_ESCAPE, VK_RIGHT},
        WindowsAndMessaging::*,
    },
};

fn norm(s: &str) -> String {
    s.split('\t').next().unwrap_or("").replace('&', "").trim_end_matches('…').trim().to_lowercase()
}

fn menus() -> Vec<HMENU> {
    windows(0, "#32768")
        .into_iter()
        .map(|w| HMENU(unsafe { SendMessageW(w, MN_GETHMENU, Some(WPARAM(0)), Some(LPARAM(0))) }.0 as _))
        .collect()
}

pub fn label(m: HMENU, i: i32) -> String {
    let mut b = [0u16; 256];
    let n = unsafe { GetMenuStringW(m, i as u32, Some(&mut b), MF_BYPOSITION) } as usize;
    String::from_utf16_lossy(&b[..n])
}

fn find(text: &str) -> Option<(HMENU, i32)> {
    let want = norm(text);
    menus().into_iter().find_map(|m| (0..unsafe { GetMenuItemCount(Some(m)) }).find(|&i| norm(&label(m, i)) == want).map(|i| (m, i)))
}

pub fn open() -> bool {
    super::wait(3, || !windows(0, "#32768").is_empty())
}

pub fn entries() -> Vec<String> {
    menus()
        .into_iter()
        .flat_map(|m| (0..unsafe { GetMenuItemCount(Some(m)) }).map(move |i| label(m, i)))
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn checked(text: &str) -> Option<bool> {
    let (m, i) = find(text)?;
    Some(unsafe { GetMenuState(m, i as u32, MF_BYPOSITION) } & MF_CHECKED.0 != 0 || label(m, i).ends_with('✓'))
}

pub fn close() {
    for _ in 0..4 {
        if windows(0, "#32768").is_empty() {
            return;
        }
        super::chord(&[VK_ESCAPE]);
    }
}

pub fn pick(path: &[&str]) -> bool {
    for text in path {
        let Some((m, i)) = poll(3, || find(text)) else {
            println!("      menu entry {text:?} not found among {:?}", entries());
            close();
            return false;
        };
        let mut r = RECT::default();
        let _ = unsafe { GetMenuItemRect(None, m, i as u32, &mut r) };
        let at = ((r.left + r.right) / 2, (r.top + r.bottom) / 2);
        let (count, sub) = (windows(0, "#32768").len(), !unsafe { GetSubMenu(m, i) }.is_invalid());
        glide((at.0 - 10, at.1), at);
        thread::sleep(Duration::from_millis(120));
        press(true);
        press(false);
        if sub && !super::wait(1, || windows(0, "#32768").len() > count) {
            super::chord(&[VK_RIGHT]);
            super::wait(1, || windows(0, "#32768").len() > count);
        }
        thread::sleep(Duration::from_millis(300));
    }
    true
}
