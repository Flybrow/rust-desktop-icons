use super::{layout, metrics, render, tabs};
use crate::{
    app::with,
    domain::{
        grid::{self, Move},
        side::Side,
    },
};
use windows::Win32::{Foundation::HWND, UI::Input::KeyboardAndMouse::*};

pub(super) fn tab(h: HWND, back: bool) {
    let Some((i, n)) = with(|a| a.fence_of(h).map(|f| (f.tab, f.tabs.len()))).flatten().filter(|(_, n)| *n > 1) else { return };
    tabs::select(h, if back { (i + n - 1) % n } else { (i + 1) % n });
}

pub(super) fn key(h: HWND, k: VIRTUAL_KEY, shift: bool) -> bool {
    let (g, cell) = (layout::geo_now(h), metrics(h).0);
    let (gw, view_h) = (g.grid_w(), g.view_h());
    let rows = (view_h / cell.max(1)).max(1);
    let m = match k {
        VK_LEFT => Move::Left,
        VK_RIGHT => Move::Right,
        VK_UP => Move::Up,
        VK_DOWN => Move::Down,
        VK_HOME => Move::Home,
        VK_END => Move::End,
        VK_PRIOR => Move::Page(-rows),
        VK_NEXT => Move::Page(rows),
        _ => return false,
    };
    let flip = (g.side == Side::Right, g.side == Side::Bottom);
    with(|a| {
        let v = a.view(h)?;
        let (n, cols) = (v.items.len(), grid::cols(gw, cell));
        let picked = |i: &usize| v.selected.contains(&v.items[*i].path);
        let at = v.cursor.filter(|i| *i < n && picked(i)).or_else(|| (0..n).find(picked));
        let to = grid::step(n, cols, at, m, flip)?;
        if !shift {
            v.selected.clear();
        }
        v.selected.insert(v.items[to].path.clone());
        v.cursor = Some(to);
        let (top, bottom) = grid::row_span(to, cols, cell);
        v.scroll = if top < v.scroll {
            top
        } else if bottom > v.scroll + view_h {
            bottom - view_h
        } else {
            v.scroll
        };
        Some(())
    });
    render(h);
    true
}
