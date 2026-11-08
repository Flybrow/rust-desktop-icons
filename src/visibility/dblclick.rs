use crate::{
    app::with,
    domain::search::{Click, double},
    shell::hide::Desk,
    tray,
    win::{buttons_down, desktop_at, inst, xy},
};
use std::{
    cell::Cell,
    sync::atomic::{AtomicU32, Ordering},
    thread,
};
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::{Input::KeyboardAndMouse::GetDoubleClickTime, WindowsAndMessaging::*},
};

pub const WM_DESKDBL: u32 = WM_APP + 22;
pub const TIMER: usize = tray::timers::DBLCLICK;

static THREAD: AtomicU32 = AtomicU32::new(0);
const STARTING: u32 = u32::MAX;

thread_local! {
    static LAST: Cell<Option<Click>> = const { Cell::new(None) };
    static PENDING: Cell<Option<(i32, i32)>> = const { Cell::new(None) };
}

unsafe extern "system" fn proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 && wp.0 as u32 == WM_LBUTTONDOWN {
        let m = unsafe { &*(lp.0 as *const MSLLHOOKSTRUCT) };
        let now = Click { time: m.time, at: (m.pt.x, m.pt.y) };
        let span = unsafe { (GetSystemMetrics(SM_CXDOUBLECLK), GetSystemMetrics(SM_CYDOUBLECLK)) };
        if !desktop_at(now.at) {
            LAST.set(None);
        } else if double(LAST.get(), now, unsafe { GetDoubleClickTime() }, span) {
            LAST.set(None);
            let packed = (now.at.0 as u16 as isize) | ((now.at.1 as u16 as isize) << 16);
            let _ = tray::hwnd().map(|t| unsafe { PostMessageW(Some(t), WM_DESKDBL, WPARAM(0), LPARAM(packed)) });
        } else {
            LAST.set(Some(now));
        }
    }
    unsafe { CallNextHookEx(None, code, wp, lp) }
}

fn watch() {
    let mut msg = MSG::default();
    let _ = unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE) };
    let hook = match unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(proc), Some(inst()), 0) } {
        Ok(h) => h,
        Err(e) => {
            let _ = THREAD.compare_exchange(STARTING, 0, Ordering::AcqRel, Ordering::Acquire);
            return crate::report::log(&format!("cannot watch desktop double-clicks: {e}"));
        }
    };
    if THREAD.compare_exchange(STARTING, unsafe { GetCurrentThreadId() }, Ordering::AcqRel, Ordering::Acquire).is_ok() {
        while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {}
    }
    let _ = unsafe { UnhookWindowsHookEx(hook) };
}

pub fn sync() {
    let on = with(|a| a.cfg.dblclick_hide) == Some(true);
    if on && THREAD.compare_exchange(0, STARTING, Ordering::AcqRel, Ordering::Acquire).is_ok() {
        thread::spawn(watch);
    } else if !on {
        stop();
    }
}

pub fn stop() {
    let t = THREAD.swap(0, Ordering::AcqRel);
    if t != 0 && t != STARTING {
        let _ = unsafe { PostThreadMessageW(t, WM_QUIT, WPARAM(0), LPARAM(0)) };
    }
}

pub fn received(lp: LPARAM) {
    PENDING.set(Some(xy(lp)));
    check();
}

pub fn check() {
    if buttons_down() {
        return tray::timers::set(TIMER, Some(50));
    }
    tray::timers::set(TIMER, None);
    let Some(p) = PENDING.take() else { return };
    if super::hidden() || !Desk::open().is_none_or(|d| d.item_at(p)) {
        super::toggle_hidden();
    }
}
