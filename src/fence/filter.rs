use super::{CARET, field::Field, layout, render, to_client, view};
use crate::{
    app::{Filter, Item, View},
    domain::search,
};
use windows::Win32::{
    Foundation::{HWND, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{GetFocus, SetFocus},
        WindowsAndMessaging::{GetCaretBlinkTime, KillTimer, SetForegroundWindow, SetTimer},
    },
};

fn narrow(f: &Filter) -> Vec<Item> {
    let names: Vec<String> = f.all.iter().map(|i| String::from_utf16_lossy(&i.name)).collect();
    search::keep(&f.query, &names).into_iter().map(|i| f.all[i].clone()).collect()
}

pub(super) fn show(v: &mut View, items: Vec<Item>) {
    match &mut v.filter {
        Some(f) => {
            f.all = items;
            v.items = narrow(f);
        }
        None => v.items = items,
    }
}

pub(super) fn clear(v: &mut View) {
    if let Some(f) = v.filter.take() {
        v.items = f.all;
        v.hover = None;
    }
}

fn changed(h: HWND, edit: impl FnOnce(&mut View)) {
    view(h, |v| {
        edit(v);
        v.hover = None;
        v.scroll = 0;
        if let Some(f) = &mut v.filter {
            f.caret = true;
            v.items = narrow(f);
        }
    });
    wake(h);
    layout::apply(h);
    render(h);
}

fn wake(h: HWND) {
    let ms = unsafe { GetCaretBlinkTime() };
    if ms != u32::MAX {
        unsafe { SetTimer(Some(h), CARET, ms, None) };
    }
}

pub(super) fn blink(h: HWND) {
    let focused = unsafe { GetFocus() } == h;
    let on = view(h, |v| v.filter.as_mut().map(|f| f.caret = focused && !f.caret)).flatten();
    if on.is_none() || !focused {
        let _ = unsafe { KillTimer(Some(h), CARET) };
    }
    if on.is_some() {
        render(h);
    }
}

pub(super) fn focus(h: HWND) {
    if view(h, |v| v.filter.as_mut().map(|f| f.caret = true)).flatten().is_some() {
        wake(h);
        render(h);
    }
}

pub(super) fn active(h: HWND) -> bool {
    view(h, |v| v.filter.is_some()) == Some(true)
}

pub(super) fn toggle(h: HWND) {
    if active(h) {
        return changed(h, clear);
    }
    changed(h, |v| v.filter = Some(Filter { query: String::new(), all: std::mem::take(&mut v.items), caret: true }));
    unsafe {
        let _ = SetForegroundWindow(h);
        let _ = SetFocus(Some(h));
    }
}

pub(super) fn bar_click(h: HWND, screen: (i32, i32)) -> bool {
    let g = layout::geo_now(h);
    let shown = active(h) || super::read(h, |f| f.active().portal.is_some()) == Some(true);
    let hit = shown && g.along_bar(to_client(h, screen)).is_some_and(|a| a >= g.length() - g.bar);
    if hit {
        toggle(h);
    }
    hit
}

pub(super) fn typed(h: HWND, wp: WPARAM) -> bool {
    let Some(c) = crate::win::typed(wp).filter(|_| active(h)) else { return false };
    changed(h, |v| v.filter.iter_mut().for_each(|f| f.query.push(c)));
    true
}

pub(super) fn back(h: HWND) -> bool {
    match view(h, |v| v.filter.as_ref().map(|f| f.query.is_empty())).flatten() {
        Some(true) => changed(h, clear),
        Some(false) => changed(h, |v| v.filter.iter_mut().for_each(|f| _ = f.query.pop())),
        None => return false,
    }
    true
}

pub(super) fn escape(h: HWND) -> bool {
    active(h) && {
        changed(h, clear);
        true
    }
}

pub(super) fn left(h: HWND) {
    if view(h, |v| v.filter.as_ref().is_some_and(|f| f.query.is_empty())) == Some(true) {
        changed(h, clear);
    }
}

pub(super) fn first(h: HWND) -> Option<std::path::PathBuf> {
    view(h, |v| v.filter.as_ref().and(v.items.first()).map(|i| i.path.clone())).flatten()
}

pub(super) fn field(h: HWND, v: &View, empty: &str) -> Option<Field> {
    let f = v.filter.as_ref()?;
    let (hint, caret) = (f.query.is_empty(), f.caret && unsafe { GetFocus() } == h);
    Some(Field { text: if hint { empty.to_string() } else { f.query.clone() }, hint, caret })
}
