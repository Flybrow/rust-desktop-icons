use super::{items, window::raise};
use crate::win::desk_host;
use std::cell::Cell;
use windows::Win32::{
    Foundation::HWND,
    UI::{Accessibility::*, WindowsAndMessaging::*},
};

pub const ADOPT: usize = crate::tray::timers::ADOPT;

thread_local! {
    static HOOKS: Cell<[isize; 2]> = const { Cell::new([0; 2]) };
    static SHELL: Cell<u32> = const { Cell::new(0) };
}

pub(super) fn owner(raised: bool) -> Option<HWND> {
    if super::window::top() || raised {
        return None;
    }
    let p = desk_host();
    if p.is_none() {
        crate::tray::timers::set(ADOPT, Some(500));
    }
    p
}

pub fn adopt() {
    let Some(p) = desk_host() else { return };
    crate::tray::timers::set(ADOPT, None);
    if super::window::top() || crate::visibility::peeking() {
        return;
    }
    let stray = items::windows().into_iter().filter(|h| unsafe { GetWindow(*h, GW_OWNER) }.ok() != Some(p));
    stray.for_each(|h| raise(h, false));
}

unsafe extern "system" fn event(_: HWINEVENTHOOK, e: u32, h: HWND, obj: i32, _: i32, _: u32, _: u32) {
    if e == EVENT_SYSTEM_FOREGROUND || (obj == OBJID_WINDOW.0 && crate::win::is_desktop(h)) {
        adopt();
    }
}

pub fn bind() {
    let Some(p) = crate::win::progman() else { return };
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(p, Some(&mut pid)) };
    if SHELL.replace(pid) == pid {
        return;
    }
    for h in HOOKS.get() {
        let _ = unsafe { UnhookWinEvent(HWINEVENTHOOK(h as _)) };
    }
    let hook = |from, to, pid| unsafe { SetWinEventHook(from, to, None, Some(event), pid, 0, WINEVENT_OUTOFCONTEXT).0 as isize };
    HOOKS.set([hook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, 0), hook(EVENT_OBJECT_SHOW, EVENT_OBJECT_HIDE, pid)]);
}
