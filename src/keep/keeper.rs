use super::live;
use crate::{
    app::{change, with},
    report,
    shell::hide::{Desk, listview},
    tray,
    win::buttons_down,
};
use std::cell::Cell;
use windows::Win32::{
    Foundation::HWND,
    UI::{Accessibility::*, WindowsAndMessaging::*},
};

pub const TICK: usize = 8;
pub const SOON: usize = 9;
const TICK_MS: u32 = 3000;
const SOON_MS: u32 = 30;
const RETRY_MS: u32 = 250;

thread_local! {
    static HOOK: Cell<(isize, isize)> = const { Cell::new((0, 0)) };
}

unsafe extern "system" fn event(_: HWINEVENTHOOK, _: u32, h: HWND, _: i32, _: i32, _: u32, _: u32) {
    if h.0 as isize == HOOK.get().1 {
        soon();
    }
}

fn unhook() {
    let (hook, _) = HOOK.replace((0, 0));
    if hook != 0 {
        let _ = unsafe { UnhookWinEvent(HWINEVENTHOOK(hook as _)) };
    }
}

fn hook() {
    let Some(lv) = listview() else { return unhook() };
    let (cur, view) = HOOK.get();
    if cur != 0 && view == lv.0 as isize {
        return;
    }
    unhook();
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(lv, Some(&mut pid)) };
    let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
    let h = unsafe { SetWinEventHook(EVENT_OBJECT_CREATE, EVENT_OBJECT_LOCATIONCHANGE, None, Some(event), pid, 0, flags) };
    HOOK.set((h.0 as isize, lv.0 as isize));
}

fn timer(id: usize, ms: Option<u32>) {
    let Some(t) = tray::hwnd() else { return };
    match ms {
        Some(ms) => unsafe {
            SetTimer(Some(t), id, ms, None);
        },
        None => {
            let _ = unsafe { KillTimer(Some(t), id) };
        }
    }
}

pub fn soon() {
    timer(SOON, Some(SOON_MS));
}

pub fn apply() {
    timer(SOON, None);
    if buttons_down() {
        return timer(SOON, Some(RETRY_MS));
    }
    if with(|a| a.cfg.keep_on_desktop) == Some(true) {
        super::reconcile();
    }
    let Some(paths) = with(|a| a.cfg.keep_on_desktop.then(|| a.cfg.pinned())).flatten().filter(|_| live()) else { return };
    timer(TICK, Some(TICK_MS));
    hook();
    let ready = Desk::open().filter(|_| HOOK.get().0 != 0);
    let Some(d) = ready else { return timer(SOON, Some(RETRY_MS)) };
    if let Some(f) = d.flags().filter(|_| with(|a| a.cfg.desk_flags.is_none()) == Some(true)) {
        change(|c| c.desk_flags = Some(f));
    }
    d.sync(&paths);
}

pub fn restore() {
    timer(TICK, None);
    timer(SOON, None);
    unhook();
    let Some(flags) = with(|a| a.cfg.desk_flags).flatten().filter(|_| live()) else { return };
    let Some(d) = Desk::open() else { return report::log("cannot reach the desktop view to show hidden icons") };
    d.sync(&[]);
    d.set_flags(flags);
    change(|c| c.desk_flags = None);
}
