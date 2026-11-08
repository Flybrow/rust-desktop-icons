use crate::{app::with, tray, win::*};
use std::cell::RefCell;
use windows::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::FW_NORMAL,
    UI::{
        Input::KeyboardAndMouse::{VK_ESCAPE, VK_RETURN},
        WindowsAndMessaging::{MSG, WM_KEYDOWN},
    },
};

type Done = Box<dyn FnOnce(String)>;

thread_local! {
    static ACTIVE: RefCell<Option<(HWND, Done)>> = const { RefCell::new(None) };
}

pub fn ask(initial: &str, done: impl FnOnce(String) + 'static) {
    finish(false);
    let Some(owner) = tray::hwnd() else { return };
    let area = work_area_at(cursor_pos());
    let (w, ht) = (sys_scale(360), sys_scale(30));
    let (x, y) = (area.left + (area.right - area.left - w) / 2, area.top + (area.bottom - area.top) / 3);
    let font = with(|a| a.font(sys_scale(15), FW_NORMAL.0)).unwrap_or_default();
    if let Some(e) = edit_popup(owner, RECT { left: x, top: y, right: x + w, bottom: y + ht }, initial, font, false) {
        ACTIVE.with(|a| *a.borrow_mut() = Some((e, Box::new(done))));
    }
}

pub fn key(msg: &MSG) -> bool {
    let k = msg.wParam.0 as u16;
    let ours = ACTIVE.with(|a| a.borrow().as_ref().is_some_and(|(e, _)| *e == msg.hwnd));
    if !ours || msg.message != WM_KEYDOWN || (k != VK_RETURN.0 && k != VK_ESCAPE.0) {
        return false;
    }
    finish(k == VK_RETURN.0);
    true
}

pub fn finish(commit: bool) {
    let Some((e, done)) = ACTIVE.with(|a| a.borrow_mut().take()) else { return };
    let text = take_text(e);
    if commit && !text.trim().is_empty() {
        done(text);
    }
}
