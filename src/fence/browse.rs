use super::{WM_CHANGED, bind, fence_of, layout, read, to_client, view};
use crate::{
    domain::{Tab, system::Sys},
    shell,
};
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::HWND;

pub(super) fn current(h: HWND) -> Option<PathBuf> {
    view(h, |v| v.browse.last().cloned()).flatten()
}

pub(super) fn tab(h: HWND, tab: Tab) -> Tab {
    let active = fence_of(h).is_some_and(|f| f.active().id == tab.id);
    match current(h).filter(|_| active) {
        Some(dir) => tab.browsing(dir),
        None => tab,
    }
}

pub(super) fn open(h: HWND, p: &Path, ctrl: bool) {
    let inside = !ctrl && openable(p) && read(h, |f| f.look.browse) == Some(true);
    if !inside {
        return shell::open(p);
    }
    enter(h, p);
}

pub(super) fn enter(h: HWND, p: &Path) {
    view(h, |v| v.browse.push(p.to_path_buf()));
    bind(h);
}

pub(super) fn forget(h: HWND) {
    view(h, |v| v.browse.clear());
}

pub(super) fn home(h: HWND) {
    if view(h, |v| !std::mem::take(&mut v.browse).is_empty()) == Some(true) {
        bind(h);
    }
}

pub(super) fn openable(p: &Path) -> bool {
    p.is_dir() && Sys::of(p).is_none()
}

pub(super) fn back(h: HWND) -> bool {
    let popped = view(h, |v| v.browse.pop().is_some()) == Some(true);
    if popped {
        bind(h);
    }
    popped
}

pub(super) fn watch(h: HWND, active: usize) -> Option<shell::Watch> {
    shell::Watch::new(h, &current(h)?, WM_CHANGED + active as u32)
}

pub(super) fn bar_click(h: HWND, screen: (i32, i32)) -> bool {
    let g = layout::geo_now(h);
    let along = g.along_bar(to_client(h, screen));
    let backed = current(h).is_some() && along.is_some_and(|a| a < g.length() / 3) && back(h);
    view(h, |v| v.backed = backed);
    backed
}

pub(super) fn bar_double(h: HWND, screen: (i32, i32)) -> bool {
    let before = view(h, |v| std::mem::take(&mut v.backed)) == Some(true);
    bar_click(h, screen) || before
}
