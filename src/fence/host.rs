use super::native;
use crate::win::{inst, register};
use std::cell::Cell;
use windows::{
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
    core::*,
};

const CLASS: PCWSTR = w!("RustDesktopIcons.MenuHost");

thread_local! {
    static HOST: Cell<isize> = const { Cell::new(0) };
    static FENCE: Cell<isize> = const { Cell::new(0) };
}

unsafe extern "system" fn proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match m {
        WM_INITMENUPOPUP | WM_DRAWITEM | WM_MEASUREITEM | WM_MENUCHAR => {
            native::forward(HWND(FENCE.get() as _), m, wp, lp).unwrap_or_else(|| unsafe { DefWindowProcW(h, m, wp, lp) })
        }
        _ => unsafe { DefWindowProcW(h, m, wp, lp) },
    }
}

pub fn register_class() {
    register(CLASS, Some(proc));
}

pub(super) fn for_menu(fence: HWND) -> HWND {
    FENCE.set(fence.0 as isize);
    let live = Some(HWND(HOST.get() as _)).filter(|h| !h.is_invalid() && unsafe { IsWindow(Some(*h)) }.as_bool());
    live.or_else(|| {
        let ex = WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE;
        let h = unsafe { CreateWindowExW(ex, CLASS, None, WS_POPUP, 0, 0, 0, 0, None, None, Some(inst()), None) }.ok()?;
        HOST.set(h.0 as isize);
        Some(h)
    })
    .unwrap_or(fence)
}
