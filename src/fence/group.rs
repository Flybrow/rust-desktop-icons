use super::{layout, read};
use crate::{
    app::{App, change, with},
    i18n::T,
    layered::glyph as g,
    win::*,
};
use std::cell::RefCell;
use windows::Win32::{
    Foundation::{HWND, RECT},
    UI::WindowsAndMessaging::{HMENU, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos},
};

const WITH: usize = 1000;
const UNGROUP: usize = 149;

struct Drag {
    start: RECT,
    mates: Vec<(HWND, RECT)>,
    moved: bool,
}

thread_local! {
    static DRAG: RefCell<Option<Drag>> = const { RefCell::new(None) };
}

fn others(a: &App, id: u64) -> Vec<(u64, String)> {
    a.cfg.fences.iter().filter(|f| f.id != id).map(|f| (f.id, f.tabs[0].title.clone())).collect()
}

pub(super) fn mates(h: HWND) -> Vec<HWND> {
    with(|a| {
        let id = a.fence_of(h)?.id;
        let ids = a.cfg.mates(id);
        Some(a.views.iter().filter(|v| ids.contains(&v.id)).map(|v| v.hwnd).collect())
    })
    .flatten()
    .unwrap_or_default()
}

pub(super) fn submenu(a: &mut App, m: HMENU, id: u64) {
    let (sub, mates) = (menu(), a.cfg.mates(id));
    for (k, (other, title)) in others(a, id).into_iter().enumerate() {
        item(sub, WITH + k, &title, mates.contains(&other));
    }
    a.sub(m, T::GroupWith, sub, g::TAB);
    if !mates.is_empty() {
        a.entry(m, UNGROUP, T::Ungroup, false, g::CLOSE);
    }
}

pub(super) fn handle(h: HWND, id: usize) -> bool {
    let Some(me) = read(h, |f| f.id) else { return false };
    match id {
        UNGROUP => change(|c| c.ungroup(me)),
        k if k >= WITH => {
            let Some(other) = with(|a| others(a, me).get(k - WITH).map(|o| o.0)).flatten() else { return false };
            change(|c| c.group_with(me, other));
        }
        _ => return false,
    }
    true
}

pub(super) fn grab(h: HWND) {
    let mates = mates(h).into_iter().map(|m| (m, window_rect(m))).collect();
    DRAG.with(|d| *d.borrow_mut() = Some(Drag { start: window_rect(h), mates, moved: false }));
}

pub(super) fn follow(left: i32, top: i32) {
    let Some(moves) = DRAG.with(|d| {
        let mut d = d.borrow_mut();
        let d = d.as_mut()?;
        let (dx, dy) = (left - d.start.left, top - d.start.top);
        d.moved |= dx != 0 || dy != 0;
        Some(d.mates.iter().map(|(m, r)| (*m, r.left + dx, r.top + dy)).collect::<Vec<_>>())
    }) else {
        return;
    };
    for (m, x, y) in moves {
        let _ = unsafe { SetWindowPos(m, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) };
    }
}

pub(super) fn release() {
    if let Some(d) = DRAG.with(|d| d.borrow_mut().take()).filter(|d| d.moved) {
        d.mates.iter().for_each(|(m, _)| layout::persist(*m));
    }
}
