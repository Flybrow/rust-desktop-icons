use super::{
    Frame,
    chrome::{self, inside},
};
use crate::{
    app::{App, with},
    win::*,
};
use windows::{
    Win32::{Foundation::*, Graphics::Gdi::ScreenToClient, UI::WindowsAndMessaging::*},
    core::*,
};

pub fn open(class: PCWSTR, ex: WINDOW_EX_STYLE, width: i32, top: impl FnOnce(&RECT) -> i32) -> Option<HWND> {
    let area = work_area_at(cursor_pos());
    let (x, y) = (area.left + (area.right - area.left) / 2, top(&area));
    let h = unsafe { CreateWindowExW(ex, class, w!("Rust Desktop Icons"), WS_POPUP, x, y, 1, 1, None, None, Some(inst()), None) }.ok()?;
    let w = scale(h, width);
    let _ = unsafe { SetWindowPos(h, None, x - w / 2, y, w, scale(h, 200), SWP_NOZORDER | SWP_NOACTIVATE) };
    Some(h)
}

pub fn follow_dpi(h: HWND, lp: LPARAM) {
    let r = unsafe { &*(lp.0 as *const RECT) };
    let _ = unsafe { SetWindowPos(h, None, r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE) };
}

pub fn show(h: HWND) {
    unsafe {
        let _ = ShowWindow(h, SW_SHOW);
        let _ = SetForegroundWindow(h);
    }
}

pub fn close_rect(h: HWND) -> RECT {
    chrome::close_rect(client_rect(h).right, |v| scale(h, v))
}

pub fn hit(h: HWND, lp: LPARAM) -> LRESULT {
    let (x, y) = xy(lp);
    let mut p = POINT { x, y };
    let _ = unsafe { ScreenToClient(h, &mut p) };
    let caption = p.y < scale(h, chrome::TOP) && !inside(&close_rect(h), (p.x, p.y));
    LRESULT(if caption { HTCAPTION } else { HTCLIENT } as isize)
}

pub fn resize(h: HWND, (w, ht): (i32, i32), y: Option<i32>) {
    let r = window_rect(h);
    if (r.right - r.left, r.bottom - r.top) == (w, ht) && y.is_none_or(|y| y == r.top) {
        return;
    }
    let flags = SWP_NOZORDER | SWP_NOACTIVATE | if y.is_none() { SWP_NOMOVE } else { SET_WINDOW_POS_FLAGS(0) };
    let _ = unsafe { SetWindowPos(h, None, r.left, y.unwrap_or(r.top), w, ht, flags) };
}

pub fn paint(h: HWND, name: &str, draw: impl FnOnce(&mut Frame, &mut App) -> Option<()>) {
    let wr = window_rect(h);
    let Some(mut frame) = Frame::new(wr.right - wr.left, wr.bottom - wr.top) else { return };
    if with(|a| draw(&mut frame, a)).flatten().is_none() {
        return;
    }
    #[cfg(debug_assertions)]
    frame.dump(name);
    #[cfg(not(debug_assertions))]
    let _ = name;
    frame.present(h, (wr.left, wr.top), 255);
}
