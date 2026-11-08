use crate::{
    app::with,
    fence,
    shell::{vdesk, vwatch},
    tray,
};
use std::cell::RefCell;
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{PostMessageW, SW_HIDE, SW_SHOWNOACTIVATE, ShowWindow, WM_APP},
};

pub const TIMER: usize = tray::timers::DESKS;
pub const WM_DESKS: u32 = WM_APP + 23;
const POLL_MS: u32 = 400;

type State = (Option<String>, Vec<String>);

thread_local! {
    static STATE: RefCell<State> = const { RefCell::new((None, vec![])) };
    static WATCH: RefCell<Option<vwatch::Watcher>> = const { RefCell::new(None) };
}

fn read() -> State {
    (vdesk::current(), vdesk::ids())
}

pub fn shown(h: HWND) -> bool {
    STATE.with(|s| {
        let s = s.borrow();
        with(|a| a.fence_of(h).map(|f| f.shown_on(s.0.as_deref(), &s.1))).flatten().unwrap_or(true)
    })
}

pub fn apply() {
    if super::hidden() {
        return;
    }
    for h in fence::hwnds() {
        let _ = unsafe { ShowWindow(h, if shown(h) { SW_SHOWNOACTIVATE } else { SW_HIDE }) };
    }
}

pub fn poll() {
    let now = read();
    if STATE.with(|s| s.replace(now.clone())) != now {
        super::end_peek();
        apply();
    }
}

pub fn sync() {
    let used = with(|a| a.cfg.fences.iter().any(|f| f.desktop.is_some())).unwrap_or(false);
    STATE.with(|s| *s.borrow_mut() = if used { read() } else { (None, vec![]) });
    WATCH.with(|w| {
        let mut w = w.borrow_mut();
        if !used {
            *w = None;
        } else if w.is_none() {
            *w = vwatch::watch(notify);
        }
    });
    let watching = WATCH.with(|w| w.borrow().is_some());
    tray::timers::set(TIMER, (used && !watching).then_some(POLL_MS));
    apply();
}

fn notify() {
    if let Some(t) = tray::hwnd() {
        let _ = unsafe { PostMessageW(Some(t), WM_DESKS, WPARAM(0), LPARAM(0)) };
    }
}

pub fn list() -> Vec<(String, Option<String>)> {
    vdesk::ids().into_iter().map(|id| (id.clone(), vdesk::name(&id))).collect()
}
