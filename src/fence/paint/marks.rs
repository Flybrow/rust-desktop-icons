use super::super::cell::{self, Cell};
use crate::{
    app::View,
    domain::{privacy::Mask, side::Geo},
    layered::{ACCENT, measure},
    render::{Canvas, flat, premul},
};
use windows::Win32::Graphics::Gdi::HFONT;

pub(super) struct Ctx<'a> {
    pub shape: Cell,
    pub icon: i32,
    pub ink: u32,
    pub rad: f32,
    pub rows: (i32, i32),
    pub s: &'a dyn Fn(i32) -> i32,
}

type Cells = [(usize, i32, i32)];

pub(super) fn highlights(c: &mut Canvas, cells: &Cells, v: &View, (font, labels): (HFONT, bool), k: &Ctx) {
    let s = k.s;
    for &(i, x, y) in cells {
        let (sel, hot) = (v.selected.contains(&v.items[i].path), v.hover == Some(i));
        if !(sel || hot) {
            continue;
        }
        let label = if labels { measure(font, &v.items[i].name, cell::label_width(k.shape, s), k.shape.flags()) } else { 0 };
        let r = cell::tile((x, y), k.shape, k.icon, label, s);
        let (fill, line) = if sel { (premul(ACCENT, 90), premul(ACCENT, 200)) } else { (premul(k.ink, 40), premul(k.ink, 60)) };
        c.rrect(r, k.rad, flat(fill), k.rows, false);
        c.rrect(r, k.rad, flat(line), k.rows, true);
    }
}

pub(super) fn masks(c: &mut Canvas, cells: &Cells, mask: Mask, named: bool, k: &Ctx) {
    let s = k.s;
    for &(_, x, y) in cells.iter().filter(|_| mask.any()) {
        if mask.icons {
            let (ix, iy) = cell::icon_at((x, y), k.shape, k.icon, s);
            c.rrect((ix, iy, ix + k.icon, iy + k.icon), s(8) as f32, flat(premul(k.ink, 46)), k.rows, false);
        }
        if mask.names && named {
            let r = cell::label_at((x, y), k.shape, k.icon, s);
            let (pad, top) = ((r.right - r.left) / 5, r.top + s(4));
            c.rrect((r.left + pad, top, r.right - pad, top + s(8)), s(4) as f32, flat(premul(k.ink, 70)), k.rows, false);
        }
    }
}

pub(super) fn band(c: &mut Canvas, g: &Geo, v: &View, k: &Ctx) {
    if let Some(((x0, y0), (x1, y1))) = v.band {
        let r = g.rect((x0.min(x1), y0.min(y1) - v.scroll, x0.max(x1), y0.max(y1) - v.scroll));
        c.rrect(r, (k.s)(2) as f32, flat(premul(ACCENT, 50)), k.rows, false);
        c.rrect(r, (k.s)(2) as f32, flat(premul(ACCENT, 200)), k.rows, true);
    }
}

pub(super) fn scrollbar(c: &mut Canvas, g: &Geo, (scroll, max, gw): (i32, i32, i32), k: &Ctx) {
    if max <= 0 {
        return;
    }
    let s = k.s;
    let avail = g.view_h();
    let th = (avail * avail / (avail + max)).max(s(24));
    let ty = scroll * (avail - th) / max;
    let r = g.rect((gw - s(7), ty + s(3), gw - s(3), ty + th - s(3)));
    c.rrect(r, s(2) as f32, flat(premul(k.ink, 110)), k.rows, false);
}
