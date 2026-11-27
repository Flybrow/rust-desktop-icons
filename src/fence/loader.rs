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
    Foundation::{HWND, LPARAM},
    System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
    UI::WindowsAndMessaging::{MSG, PM_REMOVE, PeekMessageW},
};

pub(super) const WM_LOADED: u32 = WM_CHANGED + TABS as u32;
const THUMB_BUDGET: usize = 48 << 20;

struct Batch(Vec<(u64, u64, Vec<Item>)>);

fn meta(p: &Path) -> Meta {
    let md = fs::metadata(p).ok();
    let secs =
        |t: std::io::Result<std::time::SystemTime>| t.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    let (modified, created) = md.as_ref().map_or((0, 0), |m| (secs(m.modified()), secs(m.created())));
    let (dir, size) = md.as_ref().map_or((false, 0), |m| (m.is_dir(), m.len()));
    Meta { name: store::name(p), dir, modified, created, size }
}

fn shown(tab: &Tab, p: &Path) -> bool {
    let name = store::name(p);
    !name.eq_ignore_ascii_case("desktop.ini") && (p.is_dir() || tab.shows(&name))
}

fn thumbable(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(kind::thumbable)
}

fn thumb(p: &Path, size: Option<i32>) -> Option<crate::shell::thumb::Thumb> {
    size.filter(|_| thumbable(p)).and_then(|s| crate::shell::thumb::thumbnail(p, s))
}

fn load(tab: &Tab, (thumbs, rich): (Option<i32>, bool)) -> Vec<Item> {
    let paths: Vec<PathBuf> = tab
        .system
        .iter()
        .map(|s| s.path())
        .chain(fs::read_dir(store::tab_dir(tab)).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| shown(tab, p)))
        .chain(tab.pins.iter().map(|p| p.path.clone()).filter(|p| p.exists()))
        .collect();
    let metas: Vec<Meta> = paths.iter().map(|p| meta(p)).collect();
    let mut budget = THUMB_BUDGET;
    order::arrange(&metas, (tab.sort, tab.desc), &tab.order)
        .into_iter()
        .map(|i| {
            let (icon, name) = Sys::of(&paths[i]).map_or_else(|| shell::info(&paths[i]), shell::system::info);
            let info = (rich && Sys::of(&paths[i]).is_none()).then(|| Box::new(shell::details::info(&paths[i], &metas[i])));
            let thumb = thumb(&paths[i], thumbs.filter(|_| budget > 0));
            budget = budget.saturating_sub(thumb.as_ref().map_or(0, |t| t.px.len().saturating_mul(4)));
            Item { path: paths[i].clone(), name, icon, thumb, info }
        })
        .collect()
}

pub(super) fn bare(mut items: Vec<Item>) -> Vec<Item> {
    items.iter_mut().for_each(|i| i.thumb = None);
    items
}

fn bump(gens: &mut Vec<(u64, u64)>, tab: u64) -> u64 {
    match gens.iter_mut().find(|(id, _)| *id == tab) {
        Some((_, g)) => {
            *g += 1;
            *g
        }
        None => {
            gens.push((tab, 1));
            1
        }
    }
}

pub(super) fn thumbs(h: HWND) {
    let wanted = view(h, |v| v.items.iter().any(|i| thumbable(&i.path))) == Some(true);
    if let Some(tab) = read(h, |f| f.look.thumbs.then(|| f.active().clone())).flatten().filter(|_| wanted) {
        request(h, vec![tab]);
    }
}

pub(super) fn request(h: HWND, tabs: Vec<Tab>) {
    let tabs: Vec<Tab> = tabs.into_iter().map(|t| super::browse::tab(h, t)).collect();
    let Some((look, active)) = read(h, |f| (f.look.clone(), f.active().id)) else { return };
    let Some(jobs) = with(|a| {
        let v = a.view(h)?;
        if std::mem::replace(&mut v.loading, true) {
            for t in tabs {
                bump(&mut v.gens, t.id);
                v.queued.retain(|q| q.id != t.id);
                v.queued.push(t);
            }
            return None;
        }
        Some(tabs.into_iter().map(|t| (bump(&mut v.gens, t.id), t)).collect::<Vec<_>>())
    })
    .flatten() else {
        return;
    };
    let hwnd = h.0 as isize;
    let jobs: Vec<(u64, Tab, (Option<i32>, bool))> = jobs
        .into_iter()
        .map(|(g, t)| {
            let mode = t.mode(look.list);
            let thumbs = (look.thumbs && t.id == active).then(|| super::modes::metrics_of(h, &look, mode).1);
            (g, t, (thumbs, mode.rich()))
        })
        .collect();
    thread::spawn(move || {
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        let loaded = std::panic::catch_unwind(|| jobs.into_iter().map(|(g, t, how)| (t.id, g, load(&t, how))).collect());
        crate::mail::post(HWND(hwnd as _), WM_LOADED, Batch(loaded.unwrap_or_default()));
    });
}

pub(super) fn discard(h: HWND) {
    let mut m = MSG::default();
    while unsafe { PeekMessageW(&mut m, Some(h), WM_LOADED, WM_LOADED, PM_REMOVE) }.as_bool() {
        drop(crate::mail::take::<Batch>(m.lParam));
    }
}

pub(super) fn receive(h: HWND, lp: LPARAM) {
    let Some(Batch(results)) = crate::mail::take::<Batch>(lp) else { return };
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
                let present: std::collections::HashSet<&PathBuf> = items.iter().map(|i| &i.path).collect();
                v.selected.retain(|p| present.contains(p));
                let key = Some((id, v.browse.last().cloned()));
                changed |= v.items != items || v.shown != key;
                v.shown = key;
                super::filter::show(v, items);
            } else {
                v.cache.retain(|(t, _)| *t != id);
                v.cache.push((id, bare(items)));
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
