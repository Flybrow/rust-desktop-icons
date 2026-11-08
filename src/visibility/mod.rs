mod dblclick;
pub mod hotkeys;
mod overlay;

use crate::{fence, keep, search, shell::hide::listview, tray, win::typed};
use std::cell::Cell;
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    System::Threading::GetCurrentProcessId,
    UI::{Input::KeyboardAndMouse::VK_ESCAPE, WindowsAndMessaging::*},
};

pub use dblclick::sync as sync_dblclick;
pub use overlay::register_class;

const PEEK_TIMER: usize = tray::timers::PEEK;

thread_local! {
    static HIDDEN: Cell<bool> = const { Cell::new(false) };
    static PEEKING: Cell<bool> = const { Cell::new(false) };
}

pub fn peeking() -> bool {
    PEEKING.get()
}

fn show_all(show: bool) {
    let cmd = if show { SW_SHOWNOACTIVATE } else { SW_HIDE };
    for h in fence::hwnds() {
        let _ = unsafe { ShowWindow(h, cmd) };
    }
    if let Some(lv) = listview().filter(|_| keep::live()) {
        let _ = unsafe { ShowWindow(lv, if show { SW_SHOWNA } else { SW_HIDE }) };
    }
}

pub fn reveal() {
    if HIDDEN.replace(false) {
        show_all(true);
    }
}

fn hide() {
    end_peek();
    HIDDEN.set(true);
    show_all(false);
}

fn ours(h: HWND) -> bool {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
    pid == unsafe { GetCurrentProcessId() }
}

fn timer(on: bool) {
    tray::timers::set(PEEK_TIMER, on.then_some(250));
}

pub fn toggle_peek() {
    reveal();
    if peeking() {
        return end_peek();
    }
    PEEKING.set(true);
    overlay::show();
    let fences = fence::hwnds();
    fences.iter().for_each(|h| fence::raise(*h, true));
    if let Some(h) = fences.first() {
        let _ = unsafe { SetForegroundWindow(*h) };
    }
    timer(true);
}

pub fn end_peek() {
    if !PEEKING.replace(false) {
        return;
    }
    timer(false);
    overlay::hide();
    search::close();
    fence::hwnds().into_iter().for_each(|h| fence::raise(h, false));
}

pub fn key(msg: &MSG) -> bool {
    if !peeking() || !fence::hwnds().contains(&msg.hwnd) {
        return false;
    }
    match msg.message {
        WM_KEYDOWN if msg.wParam.0 as u16 == VK_ESCAPE.0 => end_peek(),
        WM_CHAR if typed(msg.wParam).is_some() => search::open(typed(msg.wParam)),
        _ => return false,
    }
    true
}

pub fn message(m: u32, wp: WPARAM, lp: LPARAM) -> bool {
    match m {
        WM_HOTKEY => hotkeys::fired(wp.0 as i32),
        WM_TIMER if wp.0 == PEEK_TIMER => {
            let fg = unsafe { GetForegroundWindow() };
            if !fg.is_invalid() && !ours(fg) {
                end_peek();
            }
        }
        dblclick::WM_DESKDBL => dblclick::received(lp),
        WM_TIMER if wp.0 == dblclick::TIMER => dblclick::check(),
        _ => return false,
    }
    true
}

pub fn toggle_hidden() {
    if HIDDEN.get() { reveal() } else { hide() }
}

pub fn hidden() -> bool {
    HIDDEN.get()
}

fn show_icons() {
    if let Some(lv) = listview().filter(|lv| keep::live() && !unsafe { IsWindowVisible(*lv) }.as_bool()) {
        let _ = unsafe { ShowWindow(lv, SW_SHOWNA) };
    }
}

pub fn rescue() {
    if HIDDEN.get() {
        show_icons();
    }
}

pub fn recover() {
    if !HIDDEN.get() {
        show_icons();
    }
}

pub fn shutdown() {
    end_peek();
    reveal();
    dblclick::stop();
}
