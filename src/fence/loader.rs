use super::{TABS, WM_CHANGED, layout, read, view};
use crate::{
    app::{Item, with},
    domain::{
        Tab, kind,
        order::{self, Meta},
        system::Sys,
    },
    shell, store,
};
use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::UNIX_EPOCH,
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
    UI::WindowsAndMessaging::{MSG, PM_REMOVE, PeekMessageW, PostMessageW},
};

pub(super) const WM_LOADED: u32 = WM_CHANGED + TABS as u32;

struct Batch(Vec<(u64, u64, Vec<Item>)>);

fn meta(p: &Path) -> Meta {
    let md = fs::metadata(p).ok();
    let modified = md.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    let (dir, size) = md.as_ref().map_or((false, 0), |m| (m.is_dir(), m.len()));
    Meta { name: store::name(p), dir, modified, size }
}

fn shown(tab: &Tab, p: &Path) -> bool {
    let name = store::name(p);
    !name.eq_ignore_ascii_case("desktop.ini") && (p.is_dir() || tab.shows(&name))
}

fn thumb(p: &Path, size: Option<i32>) -> Option<crate::shell::thumb::Thumb> {
    let ext = p.extension()?.to_str()?;
    size.filter(|_| kind::thumbable(ext)).and_then(|s| crate::shell::thumb::thumbnail(p, s))
}

fn load(tab: &Tab, thumbs: Option<i32>) -> Vec<Item> {
    let paths: Vec<PathBuf> = tab
        .system
        .iter()
        .map(|s| s.path())
        .chain(fs::read_dir(store::tab_dir(tab)).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| shown(tab, p)))
        .chain(tab.pins.iter().map(|p| p.path.clone()).filter(|p| p.exists()))
        .collect();
    let metas: Vec<Meta> = paths.iter().map(|p| meta(p)).collect();
    order::arrange(&metas, tab.sort, &tab.order)
        .into_iter()
        .map(|i| {
            let (icon, name) = Sys::of(&paths[i]).map_or_else(|| shell::info(&paths[i]), shell::system::info);
            Item { path: paths[i].clone(), name, icon, thumb: thumb(&paths[i], thumbs) }
        })
        .collect()
}

pub(super) fn request(h: HWND, tabs: Vec<Tab>) {
    let tabs: Vec<Tab> = tabs.into_iter().map(|t| super::browse::tab(h, t)).collect();
    let Some(jobs) = with(|a| {
        let v = a.view(h)?;
        if std::mem::replace(&mut v.loading, true) {
            for t in tabs {
                v.queued.retain(|q| q.id != t.id);
                v.queued.push(t);
            }
            return None;
        }
        Some(
            tabs.into_iter()
                .map(|t| {
                    let g = match v.gens.iter_mut().find(|(id, _)| *id == t.id) {
                        Some((_, g)) => {
                            *g += 1;
                            *g
                        }
                        None => {
                            v.gens.push((t.id, 1));
                            1
                        }
                    };
                    (g, t)
                })
                .collect::<Vec<_>>(),
        )
    })
    .flatten() else {
        return;
    };
    let hwnd = h.0 as isize;
    let thumbs = read(h, |f| f.look.thumbs).unwrap_or(false).then(|| super::metrics(h).1);
    thread::spawn(move || {
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        let loaded = std::panic::catch_unwind(|| jobs.into_iter().map(|(g, t)| (t.id, g, load(&t, thumbs))).collect());
        let batch = Box::new(Batch(loaded.unwrap_or_default()));
        let raw = Box::into_raw(batch);
        if unsafe { PostMessageW(Some(HWND(hwnd as _)), WM_LOADED, WPARAM(0), LPARAM(raw as isize)) }.is_err() {
            drop(unsafe { Box::from_raw(raw) });
        }
    });
}

pub(super) fn discard(h: HWND) {
    let mut m = MSG::default();
    while unsafe { PeekMessageW(&mut m, Some(h), WM_LOADED, WM_LOADED, PM_REMOVE) }.as_bool() {
        drop(unsafe { Box::from_raw(m.lParam.0 as *mut Batch) });
    }
}

pub(super) fn receive(h: HWND, lp: LPARAM) {
    let Batch(results) = *unsafe { Box::from_raw(lp.0 as *mut Batch) };
    let next = view(h, |v| {
        v.loading = false;
        std::mem::take(&mut v.queued)
    })
    .unwrap_or_default();
    let Some(active) = read(h, |f| f.active().id) else { return };
    let changed = with(|a| {
        let v = a.view(h)?;
        let mut changed = false;
        for (id, g, items) in results {
            if !v.gens.iter().any(|&(t, cur)| t == id && cur == g) {
                continue;
            }
            if id == active {
                v.cache.retain(|(t, _)| *t != id);
                v.selected.retain(|p| items.iter().any(|i| &i.path == p));
                let key = Some((id, v.browse.last().cloned()));
                changed |= v.items != items || v.shown != key;
                v.shown = key;
                v.items = items;
            } else {
                v.cache.retain(|(t, _)| *t != id);
                v.cache.push((id, items));
            }
        }
        Some(changed)
    })
    .flatten();
    if changed != Some(false) {
        layout::apply(h);
    }
    if !next.is_empty() {
        request(h, next);
    }
}
