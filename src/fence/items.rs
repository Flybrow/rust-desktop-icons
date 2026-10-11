use super::{fence_of, layout, loader, metrics, render, update, view};
use crate::app::with;
use std::path::PathBuf;
use windows::Win32::Foundation::HWND;

pub fn reload(h: HWND) {
    if let Some(f) = fence_of(h) {
        loader::request(h, f.tabs);
    }
}

pub(super) fn refresh(h: HWND) {
    let dirty = view(h, |v| std::mem::take(&mut v.dirty)).unwrap_or_default();
    if let Some(f) = fence_of(h) {
        loader::request(h, f.tabs.into_iter().enumerate().filter(|(i, _)| dirty.contains(i)).map(|(_, t)| t).collect());
    }
}

pub fn windows() -> Vec<HWND> {
    with(|a| a.views.iter().map(|v| v.hwnd).collect()).unwrap_or_default()
}

pub fn reload_all() {
    windows().into_iter().for_each(reload);
}

pub(super) fn set_icon(h: HWND, px: i32) {
    update(h, |f| f.look.icon = px);
}

pub(super) fn index_at(h: HWND, (x, y): (i32, i32)) -> Option<usize> {
    let (g, cell) = (layout::geo_now(h), metrics(h).0);
    let (x0, y0, x1, y1) = g.body();
    let (cx, cy) = g.to_content((x, y));
    with(|a| {
        let v = a.view(h)?;
        ((x0..x1).contains(&x) && (y0..y1).contains(&y))
            .then(|| crate::domain::grid::index_at((cx, cy + v.scroll), g.grid_w(), cell, v.items.len()))?
    })
    .flatten()
}

pub(super) fn item_at(h: HWND, p: (i32, i32)) -> Option<PathBuf> {
    let i = index_at(h, p)?;
    with(|a| a.view(h)?.items.get(i).map(|it| it.path.clone())).flatten()
}

pub(super) fn hover(h: HWND, i: Option<usize>) {
    if view(h, |v| std::mem::replace(&mut v.hover, i) != i) == Some(true) {
        render(h);
    }
}
