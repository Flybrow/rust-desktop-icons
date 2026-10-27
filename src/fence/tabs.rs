use super::{TABS, WM_CHANGED, actions, fence_of, layout, reload, to_client, update, view};
use crate::{
    app::with,
    domain::{Tab, grid, system::Sys},
    i18n::T,
    keep, report, shell, store,
};
use std::path::PathBuf;
use windows::Win32::{Foundation::*, UI::Input::KeyboardAndMouse::DragDetect};

pub fn bind(h: HWND) {
    let Some((tabs, keep)) = with(|a| Some((a.fence_of(h)?.tabs.clone(), a.cfg.keep_on_desktop))).flatten() else { return };
    layout::orient(h);
    view(h, |v| v.watches.clear());
    let mut watches = vec![];
    for (i, t) in tabs.iter().enumerate().take(TABS) {
        let msg = WM_CHANGED + i as u32;
        let dir = store::tab_dir(t);
        match shell::Watch::new(h, &dir, msg) {
            Some(w) => watches.push(w),
            None if !keep => report::log(&format!("cannot watch {}", dir.display())),
            None => {}
        }
        let desks = if (keep && t.portal.is_none()) || !t.pins.is_empty() { shell::desktops() } else { vec![] };
        let recycle = t.system.contains(&Sys::RecycleBin).then(|| Sys::RecycleBin.path());
        watches.extend(desks.iter().chain(recycle.as_ref()).filter_map(|d| shell::Watch::new(h, d, msg)));
    }
    view(h, |v| (v.watches, v.scroll) = (watches, 0));
    reload(h);
}

pub(super) fn at(h: HWND, p: (i32, i32)) -> Option<usize> {
    let (n, g) = (fence_of(h)?.tabs.len(), layout::geo_now(h));
    g.along_bar(p).filter(|_| n > 1).and_then(|a| grid::split(g.length(), n, a))
}

pub(super) fn click(h: HWND, screen: (i32, i32)) -> bool {
    let Some(i) = at(h, to_client(h, screen)) else { return false };
    if unsafe { DragDetect(h, POINT { x: screen.0, y: screen.1 }) }.as_bool() {
        return false;
    }
    select(h, i);
    true
}

pub(super) fn hover(h: HWND, screen: (i32, i32)) {
    if let Some(i) = at(h, to_client(h, screen)) {
        select(h, i);
    }
}

pub(super) fn select(h: HWND, i: usize) {
    let hit = with(|a| {
        let f = a.fence_of(h)?;
        if f.tab == i || i >= f.tabs.len() {
            return None;
        }
        let (old, new) = (f.active().id, f.tabs[i].id);
        f.tab = i;
        let v = a.view(h)?;
        let items = std::mem::take(&mut v.items);
        v.cache.push((old, items));
        v.scroll = 0;
        let found = v.cache.iter().position(|(id, _)| *id == new).map(|k| v.cache.remove(k).1);
        let hit = found.is_some();
        v.items = found.unwrap_or_default();
        Some(hit)
    })
    .flatten();
    match hit {
        Some(true) => layout::apply(h),
        Some(false) => reload(h),
        None => {}
    }
}

fn add(h: HWND, portal: Option<PathBuf>) {
    let title = portal.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned());
    let dir = with(|a| {
        let (id, name) = (a.cfg.next_id(), a.t(T::NewTab));
        let f = a.fence_of(h)?;
        f.tabs.push(Tab { id, title: title.unwrap_or_else(|| name.into()), portal, ..Tab::default() });
        f.tab = f.tabs.len() - 1;
        let dir = store::tab_dir(f.active());
        a.save();
        Some(dir)
    });
    report::failures(T::ErrSave, &store::ensure(&dir.flatten().into_iter().collect::<Vec<_>>()));
    update(h, |_| {});
    bind(h);
}

pub(super) fn new_tab(h: HWND) {
    add(h, None);
}

pub(super) fn new_portal(h: HWND) {
    if let Some(dir) = shell::pick_folder(h) {
        add(h, Some(dir));
    }
}

pub(super) fn remove(h: HWND) {
    let Some(f) = fence_of(h).filter(|f| f.tabs.len() > 1) else { return };
    let tab = f.active().clone();
    if tab.portal.is_none() && !actions::confirm(h, T::ConfirmDeleteTab) {
        return;
    }
    if tab.portal.is_none() {
        let errors = actions::evacuate(&store::tab_dir(&tab));
        if !errors.is_empty() {
            report::failures(T::ErrDelete, &errors);
            return reload(h);
        }
    }
    update(h, |f| {
        f.tabs.retain(|t| t.id != tab.id);
        f.tab = f.tab.min(f.tabs.len() - 1);
    });
    keep::soon();
    super::system::released(std::slice::from_ref(&tab));
    bind(h);
}
