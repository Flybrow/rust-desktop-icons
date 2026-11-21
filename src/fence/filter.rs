use super::{layout, render, to_client, view};
use crate::{
    app::{Filter, Item, View},
    domain::search,
};
use windows::Win32::{
    Foundation::{HWND, WPARAM},
    UI::{Input::KeyboardAndMouse::SetFocus, WindowsAndMessaging::SetForegroundWindow},
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
        if let Some(f) = &v.filter {
            v.items = narrow(f);
        }
    });
    layout::apply(h);
    render(h);
}

pub(super) fn active(h: HWND) -> bool {
    view(h, |v| v.filter.is_some()) == Some(true)
}

pub(super) fn toggle(h: HWND) {
    if active(h) {
        return changed(h, clear);
    }
    changed(h, |v| v.filter = Some(Filter { query: String::new(), all: std::mem::take(&mut v.items) }));
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

pub(super) fn hint(v: &View, empty: &str) -> Option<String> {
    v.filter.as_ref().map(|f| if f.query.is_empty() { empty.to_string() } else { f.query.clone() })
}
