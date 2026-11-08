use crate::win::{inst, register_with};
use std::cell::Cell;
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::{BLACK_BRUSH, GetStockObject, HBRUSH},
        UI::WindowsAndMessaging::*,
    },
    core::*,
};

const CLASS: PCWSTR = w!("RustDesktopIcons.Overlay");
const DIM: u8 = 120;

thread_local! {
    static OVERLAY: Cell<isize> = const { Cell::new(0) };
}

unsafe extern "system" fn proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match m {
        WM_LBUTTONDOWN | WM_RBUTTONDOWN => super::end_peek(),
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
        _ => return unsafe { DefWindowProcW(h, m, wp, lp) },
    }
    LRESULT(0)
}

pub fn register_class() {
    register_with(CLASS, Some(proc), HBRUSH(unsafe { GetStockObject(BLACK_BRUSH) }.0));
}

pub fn show() {
    let (x, y, w, h) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    let ex = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_NOACTIVATE;
    let Ok(hwnd) = (unsafe { CreateWindowExW(ex, CLASS, None, WS_POPUP, x, y, w, h, None, None, Some(inst()), None) }) else { return };
    let _ = unsafe { SetLayeredWindowAttributes(hwnd, COLORREF(0), DIM, LWA_ALPHA) };
    let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
    hide();
    OVERLAY.set(hwnd.0 as isize);
}

pub fn hide() {
    let h = OVERLAY.replace(0);
    if h != 0 {
        let _ = unsafe { DestroyWindow(HWND(h as _)) };
    }
}
