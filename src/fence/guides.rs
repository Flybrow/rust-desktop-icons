use crate::{domain::snap::Guide, layered::ACCENT, win::*};
use std::cell::RefCell;
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::{CreateSolidBrush, HBRUSH},
        UI::WindowsAndMessaging::*,
    },
    core::*,
};

const CLASS: PCWSTR = w!("RustDesktopIcons.Guide");

thread_local! {
    static LINES: RefCell<Vec<isize>> = const { RefCell::new(vec![]) };
}

unsafe extern "system" fn proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(h, m, wp, lp) }
}

pub fn register_class() {
    register_with(CLASS, Some(proc), HBRUSH(unsafe { CreateSolidBrush(COLORREF(ACCENT)) }.0));
}

fn line(owner: HWND) -> Option<HWND> {
    let ex = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
    let h = unsafe { CreateWindowExW(ex, CLASS, None, WS_POPUP, 0, 0, 1, 1, Some(owner), None, Some(inst()), None) }.ok()?;
    let _ = unsafe { SetLayeredWindowAttributes(h, COLORREF(0), 230, LWA_ALPHA) };
    Some(h)
}

pub(super) fn show(owner: HWND, guides: &[Guide]) {
    let have = LINES.with(|l| l.borrow().len());
    let fresh: Vec<isize> = (have..guides.len()).filter_map(|_| line(owner)).map(|h| h.0 as isize).collect();
    let lines = LINES.with(|l| {
        let mut l = l.borrow_mut();
        l.extend(fresh);
        l.clone()
    });
    let t = scale(owner, 2);
    for (i, raw) in lines.into_iter().enumerate() {
        let h = HWND(raw as _);
        let Some(g) = guides.get(i) else {
            let _ = unsafe { ShowWindow(h, SW_HIDE) };
            continue;
        };
        let (x0, y0, x1, y1) = (g.from.0.min(g.to.0), g.from.1.min(g.to.1), g.from.0.max(g.to.0), g.from.1.max(g.to.1));
        let (x, y) = (x0 - if x0 == x1 { t / 2 } else { 0 }, y0 - if y0 == y1 { t / 2 } else { 0 });
        let flags = SWP_NOACTIVATE | SWP_NOZORDER | SWP_SHOWWINDOW;
        let _ = unsafe { SetWindowPos(h, None, x, y, (x1 - x0).max(t), (y1 - y0).max(t), flags) };
    }
}

pub(super) fn hide() {
    for raw in LINES.with(|l| std::mem::take(&mut *l.borrow_mut())) {
        let _ = unsafe { DestroyWindow(HWND(raw as _)) };
    }
}
