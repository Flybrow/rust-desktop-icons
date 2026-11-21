use crate::{app::with, domain::privacy::Reveal, fence};
use std::cell::Cell;
use windows::Win32::{
    Foundation::HWND,
    UI::{Accessibility::*, WindowsAndMessaging::*},
};

thread_local! {
    static HOOK: Cell<isize> = const { Cell::new(0) };
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
}

pub fn desktop_active() -> bool {
    ACTIVE.get()
}

fn on_desktop(h: HWND) -> bool {
    let mut name = [0u16; 32];
    let n = unsafe { GetClassNameW(h, &mut name) } as usize;
    matches!(String::from_utf16_lossy(&name[..n]).as_str(), "Progman" | "WorkerW") || fence::hwnds().contains(&h)
}

fn refresh() {
    let now = on_desktop(unsafe { GetForegroundWindow() });
    if ACTIVE.replace(now) != now {
        fence::hwnds().into_iter().for_each(fence::render);
    }
}

unsafe extern "system" fn event(_: HWINEVENTHOOK, _: u32, _: HWND, _: i32, _: i32, _: u32, _: u32) {
    refresh();
}

pub fn sync(repaint: bool) {
    let want = with(|a| a.cfg.privacy.on && a.cfg.privacy.reveal == Reveal::Desktop) == Some(true);
    let hook = HOOK.get();
    if want && hook == 0 {
        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        let h = unsafe { SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(event), 0, 0, flags) };
        HOOK.set(h.0 as isize);
    } else if !want && hook != 0 {
        let _ = unsafe { UnhookWinEvent(HWINEVENTHOOK(hook as _)) };
        HOOK.set(0);
    }
    ACTIVE.set(on_desktop(unsafe { GetForegroundWindow() }));
    if repaint {
        fence::hwnds().into_iter().for_each(fence::render);
    }
}
