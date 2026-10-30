use crate::win::{inst, register};
use std::cell::Cell;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT, PaintDesktop},
        UI::WindowsAndMessaging::*,
    },
    core::w,
};

const CLASS: windows::core::PCWSTR = w!("RustDesktopIcons.Cover");

thread_local! {
    static COVER: Cell<isize> = const { Cell::new(0) };
}

pub fn register_class() {
    register(CLASS, Some(proc));
}

fn current() -> Option<HWND> {
    Some(HWND(COVER.get() as _)).filter(|h| !h.is_invalid() && unsafe { IsWindow(Some(*h)) }.as_bool())
}

pub fn show(r: RECT) {
    let h = current().or_else(|| unsafe {
        let owner = FindWindowW(w!("Progman"), None).ok();
        let ex = WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        let h = CreateWindowExW(ex, CLASS, None, WS_POPUP, r.left, r.top, 1, 1, owner, None, Some(inst()), None).ok()?;
        COVER.set(h.0 as isize);
        Some(h)
    });
    let Some(h) = h else { return };
    let flags = SWP_NOACTIVATE | SWP_SHOWWINDOW;
    let _ = unsafe { SetWindowPos(h, Some(HWND_BOTTOM), r.left, r.top, r.right - r.left, r.bottom - r.top, flags) };
    let _ = unsafe { InvalidateRect(Some(h), None, false) };
}

pub fn hide() {
    if let Some(h) = current() {
        let _ = unsafe { DestroyWindow(h) };
    }
    COVER.set(0);
}

unsafe extern "system" fn proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match m {
        WM_PAINT => unsafe {
            let mut ps = PAINTSTRUCT::default();
            let dc = BeginPaint(h, &mut ps);
            let _ = PaintDesktop(dc);
            let _ = EndPaint(h, &ps);
        },
        WM_ERASEBKGND => return LRESULT(1),
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
        WM_SETTINGCHANGE | WM_DISPLAYCHANGE => {
            let _ = unsafe { InvalidateRect(Some(h), None, false) };
        }
        WM_WINDOWPOSCHANGING => {
            let p = unsafe { &mut *(lp.0 as *mut WINDOWPOS) };
            if (p.flags & SWP_NOZORDER).0 == 0 {
                p.hwndInsertAfter = HWND_BOTTOM;
            }
        }
        _ => return unsafe { DefWindowProcW(h, m, wp, lp) },
    }
    LRESULT(0)
}
