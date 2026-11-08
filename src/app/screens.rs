use super::{edit, rebuild, with};
use crate::{domain::screens::Screen, keep};
use windows::{
    Win32::{
        Foundation::{LPARAM, RECT},
        Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO},
    },
    core::BOOL,
};

pub const TIMER: usize = crate::tray::timers::SCREENS;
const SETTLE_MS: u32 = 1500;

fn rect(r: RECT) -> [i32; 4] {
    [r.left, r.top, r.right, r.bottom]
}

unsafe extern "system" fn collect(m: HMONITOR, _: HDC, _: *mut RECT, lp: LPARAM) -> BOOL {
    let out = unsafe { &mut *(lp.0 as *mut Vec<Screen>) };
    let mut mi = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
    if unsafe { GetMonitorInfoW(m, &mut mi) }.as_bool() {
        out.push(Screen { full: rect(mi.rcMonitor), work: rect(mi.rcWork) });
    }
    true.into()
}

fn current() -> Vec<Screen> {
    let mut out: Vec<Screen> = vec![];
    let _ = unsafe { EnumDisplayMonitors(None, None, Some(collect), LPARAM(&mut out as *mut _ as isize)) };
    out.sort_by_key(|s| (s.full[0], s.full[1]));
    out
}

pub fn adapt() -> bool {
    let now = current();
    with(|a| a.cfg.screens != now).unwrap_or(false) && edit(|a| a.cfg.adapt(&now)).flatten() == Some(true)
}

pub fn changed() {
    crate::tray::timers::set(TIMER, Some(SETTLE_MS));
}

pub fn settled() {
    crate::tray::timers::set(TIMER, None);
    if adapt() {
        rebuild();
        keep::soon();
    }
}
