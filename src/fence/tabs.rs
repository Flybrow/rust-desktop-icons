use super::{TABS, WM_CHANGED, actions, fence_of, layout, loader, read, reload, to_client, update, update_tab, view, window};
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
    view(h, |v| {
        v.watches.clear();
        v.cache.retain(|(id, _)| tabs.iter().any(|t| t.id == *id));
        v.gens.retain(|(id, _)| tabs.iter().any(|t| t.id == *id));
    });
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
    let active = with(|a| a.fence_of(h).map(|f| f.tab)).flatten().unwrap_or(0);
    watches.extend(super::browse::watch(h, active));
    view(h, |v| (v.watches, v.scroll) = (watches, 0));
    reload(h);
}

pub(super) fn at(h: HWND, p: (i32, i32)) -> Option<usize> {
    let (n, g) = (read(h, |f| f.tabs.len())?, layout::geo_now(h));
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
    let seg = at(h, to_client(h, screen));
    let entered = view(h, |v| std::mem::replace(&mut v.bar_tab, seg) != seg && v.edit.is_none()) == Some(true);
    if let Some(i) = seg.filter(|_| entered && super::browse::current(h).is_none()) {
        select(h, i);
    }
}

pub(super) fn off_bar(h: HWND) {
    view(h, |v| v.bar_tab = None);
}

pub(super) fn select(h: HWND, i: usize) {
    let from = fence_of(h).and_then(|f| super::anim::extent(h, &f));
    let hit = with(|a| {
        let f = a.fence_of(h)?;
        if f.tab == i || i >= f.tabs.len() {
            return None;
        }
        let (old, new) = (f.active().id, f.tabs[i].id);
        f.tab = i;
        let v = a.view(h)?;
        let browsed = !std::mem::take(&mut v.browse).is_empty();
        super::filter::clear(v);
        let items = std::mem::take(&mut v.items);
        v.cache.retain(|(id, _)| *id != old);
        if !browsed {
            v.cache.push((old, loader::bare(items)));
        }
        (v.scroll, v.floor) = (0, 0);
        let found = v.cache.iter().position(|(id, _)| *id == new).map(|k| v.cache.remove(k).1);
        let hit = found.is_some();
        v.items = found.unwrap_or_default();
        Some((hit, browsed))
    })
    .flatten();
    match hit {
        Some((_, true)) => bind(h),
        Some((true, _)) => {
            match from {
                Some(from) => super::anim::grow(h, from),
                None => layout::apply(h),
            }
            loader::thumbs(h);
        }
        Some((false, _)) => reload(h),
        None => {}
    }
}

fn add(h: HWND, portal: Option<PathBuf>) {
    let title = portal.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned());
    let Some(dir) = crate::app::edit(|a| {
        let (id, name) = (a.cfg.next_id(), a.t(T::NewTab));
        let f = a.fence_of(h)?;
        f.add_tab(Tab { id, title: title.unwrap_or_else(|| name.into()), portal, ..Tab::default() }).then(|| store::tab_dir(f.active()))
    })
    .flatten() else {
        return;
    };
    report::failures(T::ErrSave, &store::ensure(&[dir]));
    update(h, |_| {});
    super::browse::forget(h);
    bind(h);
}

pub(super) fn new_tab(h: HWND) {
    add(h, None);
}

pub(super) fn new_portal(h: HWND) {
    if let Some(dir) = shell::pick::folder(h) {
        add(h, Some(dir));
    }
}

pub(super) fn filter(h: HWND) {
    let Some((current, fence, tab)) = read(h, |f| (f.active().filter.clone(), f.id, f.active().id)) else { return };
    crate::prompt::ask_any(&current, move |text| {
        let Some(h) = window(fence) else { return };
        update_tab(h, tab, |t| t.filter = text.trim().to_string());
        reload(h);
    });
}

pub(super) fn remove(h: HWND) {
    let Some((fence, id, portal)) = read(h, |f| (f.tabs.len() > 1).then(|| (f.id, f.active().id, f.active().portal.is_some()))).flatten()
    else {
        return;
    };
    if !portal && !crate::report::confirm(h, T::ConfirmDeleteTab) {
        return;
    }
    let Some((h, tab)) = window(fence).and_then(|h| {
        let f = fence_of(h).filter(|f| f.tabs.len() > 1)?;
        Some((h, f.tabs.into_iter().find(|t| t.id == id)?))
    }) else {
        return;
    };
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
    super::browse::forget(h);
    bind(h);
}
