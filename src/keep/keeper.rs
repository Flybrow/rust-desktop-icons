use super::{inject::Push, live};
use crate::{
    app::{change, with},
    report,
    shell::hide::{Desk, listview},
    tray,
    win::buttons_down,
};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, RECT, WPARAM},
    UI::{Accessibility::*, WindowsAndMessaging::*},
};

pub const TICK: usize = tray::timers::KEEP_TICK;
pub const SOON: usize = tray::timers::KEEP_SOON;
const TICK_MS: u32 = 3000;
const SOON_MS: u32 = 30;
const RETRY_MS: u32 = 250;
const ABSENT_MS: u32 = 100;

type Reveal = (Vec<PathBuf>, (i32, i32));

thread_local! {
    static HOOK: Cell<(isize, isize)> = const { Cell::new((0, 0)) };
    static QUEUED: Cell<bool> = const { Cell::new(false) };
    static STACK: Cell<Option<(i32, i32)>> = const { Cell::new(None) };
    static COVER: Cell<Option<RECT>> = const { Cell::new(None) };
    static REVEAL: RefCell<Option<Reveal>> = const { RefCell::new(None) };
}

fn union(a: RECT, b: RECT) -> RECT {
    RECT { left: a.left.min(b.left), top: a.top.min(b.top), right: a.right.max(b.right), bottom: a.bottom.max(b.bottom) }
}

unsafe extern "system" fn event(_: HWINEVENTHOOK, _: u32, h: HWND, _: i32, _: i32, _: u32, _: u32) {
    if h.0 as isize == HOOK.get().1 && !QUEUED.replace(true) {
        soon();
    }
}

pub fn wake() {
    let _ = tray::hwnd().map(|t| unsafe { PostMessageW(Some(t), WM_TIMER, WPARAM(SOON), LPARAM(0)) });
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
    let tid = unsafe { GetWindowThreadProcessId(lv, Some(&mut pid)) };
    if tid == 0 {
        return;
    }
    let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
    let h = unsafe { SetWinEventHook(EVENT_OBJECT_CREATE, EVENT_OBJECT_LOCATIONCHANGE, None, Some(event), pid, tid, flags) };
    HOOK.set((h.0 as isize, lv.0 as isize));
}

use tray::timers::set as timer;

pub fn soon() {
    after(SOON_MS);
}

pub fn after(ms: u32) {
    timer(SOON, Some(ms));
}

pub fn reveal(paths: Vec<PathBuf>, at: (i32, i32)) {
    REVEAL.set(Some((paths, at)));
    apply();
}

fn revealed() {
    if let Some((paths, at)) = REVEAL.take() {
        let _ = Desk::open().map(|d| d.show_at(&paths, at));
    }
}

fn let_go() {
    timer(TICK, (!super::inject::stop()).then_some(TICK_MS));
}

pub fn apply() {
    timer(SOON, None);
    QUEUED.set(false);
    if buttons_down() {
        return timer(SOON, Some(RETRY_MS));
    }
    if with(|a| a.cfg.keep_on_desktop) == Some(true) {
        super::reconcile();
    }
    let Some(paths) = with(|a| a.cfg.keep_on_desktop.then(|| a.cfg.pinned())).flatten().filter(|_| live()) else {
        REVEAL.set(None);
        return let_go();
    };
    timer(TICK, Some(TICK_MS));
    let paths = match super::inject::push(&paths) {
        Push::Done(n) if n >= paths.len() => {
            unstack();
            return revealed();
        }
        Push::Done(n) => paths[n..].to_vec(),
        Push::Pending => return,
        Push::Absent => return timer(SOON, Some(ABSENT_MS)),
        Push::Failed => paths,
    };
    hook();
    let ready = Desk::open().filter(|_| HOOK.get().0 != 0);
    let Some(d) = ready else { return timer(SOON, Some(RETRY_MS)) };
    if let Some(f) = d.flags().filter(|_| with(|a| a.cfg.desk_flags.is_none()) == Some(true)) {
        change(|c| c.desk_flags = Some(f));
    }
    match d.sync(&paths, STACK.get(), &crate::fence::footprints()) {
        Some((stack, r)) => {
            let moved = STACK.replace(Some(stack)).is_some_and(|p| p != stack);
            let old = COVER.replace(Some(r)).filter(|_| moved);
            super::cover::show(old.map_or(r, |o| union(o, r)));
            if old.is_some() {
                timer(SOON, Some(RETRY_MS));
            }
        }
        None => {
            COVER.set(None);
            super::cover::hide();
        }
    }
    revealed();
}

fn unstack() {
    unhook();
    super::cover::hide();
    STACK.set(None);
    COVER.set(None);
    let Some(flags) = with(|a| a.cfg.desk_flags).flatten().filter(|_| live()) else { return };
    let Some(d) = Desk::open() else { return report::log("cannot reach the desktop view to show hidden icons") };
    d.sync(&[], None, &[]);
    d.set_flags(flags);
    change(|c| c.desk_flags = None);
}

pub fn restore() {
    timer(SOON, None);
    let_go();
    unstack();
}
