use super::{BORDER, TITLE, fence_of, flagged, metrics, read, render, update, view, xy};
use crate::{
    domain::{
        Fence, Zone, anim, grid,
        side::{self, Geo, Side},
        zone,
    },
    win::*,
};
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

const MARGIN: i32 = 16;
const GAP: i32 = 4;

fn progress(h: HWND, f: &Fence) -> (f32, bool) {
    view(h, |v| (v.unroll, v.opening)).unwrap_or((if f.rolled { 0. } else { 1. }, false))
}

pub(super) fn bar(h: HWND) -> i32 {
    scale(h, TITLE)
}

pub(super) fn side(h: HWND) -> Side {
    view(h, |v| v.side).unwrap_or_default()
}

pub(super) fn geo(h: HWND, size: (i32, i32)) -> Geo {
    Geo { side: side(h), size, bar: bar(h), gap: scale(h, GAP) }
}

pub(super) fn geo_now(h: HWND) -> Geo {
    let r = client_rect(h);
    geo(h, (r.right, r.bottom))
}

fn fit(h: HWND, f: &Fence, max: i32) -> i32 {
    let n = view(h, |v| v.items.len()).unwrap_or(0);
    let (cell, min) = (metrics(h).0, scale(h, Fence::MIN.1));
    match (f.vertical, f.look.auto_height) {
        (false, false) => f.h,
        (true, false) => f.w,
        (false, true) => grid::fit(n, f.w, cell, scale(h, TITLE + GAP + 8), (min, max)),
        (true, true) => grid::fit_cols(n, f.h, cell, scale(h, TITLE + GAP + 4), (min, max)),
    }
}

fn spot(h: HWND, f: &Fence) -> (i32, (i32, i32)) {
    let t = bar(h);
    let (w, ht) = if f.vertical { (t, f.h) } else { (f.w, t) };
    let work = work_area(&RECT { left: f.x, top: f.y, right: f.x + w, bottom: f.y + ht });
    if f.vertical { (f.x, (work.left, work.right)) } else { (f.y, (work.top, work.bottom)) }
}

pub(super) fn orient(h: HWND) {
    let Some(f) = fence_of(h) else { return };
    if view(h, |v| if f.rolled { v.unroll > 0. } else { v.unroll < 1. }) != Some(false) {
        return;
    }
    let (pos, span) = spot(h, &f);
    let far = (f.rolled || f.vertical) && side::far(pos, bar(h), fit(h, &f, i32::MAX), span, scale(h, MARGIN));
    view(h, |v| v.side = Side::pick(f.vertical, far));
}

pub(super) fn full(h: HWND, f: &Fence) -> i32 {
    let (pos, (lo, hi)) = spot(h, f);
    let room = if side(h).far() { pos + bar(h) - lo } else { hi - pos };
    fit(h, f, room - scale(h, MARGIN))
}

fn shape(f: &Fence, extent: i32) -> (i32, i32) {
    if f.vertical { (extent, f.h) } else { (f.w, extent) }
}

pub(super) fn open_size(h: HWND, f: &Fence) -> (i32, i32) {
    shape(f, full(h, f))
}

pub(super) fn size(h: HWND, f: &Fence) -> (i32, i32) {
    let (p, opening) = progress(h, f);
    shape(f, anim::height(bar(h), full(h, f), p, opening))
}

pub(super) fn origin(h: HWND, f: &Fence, (w, ht): (i32, i32)) -> (i32, i32) {
    let t = bar(h);
    match side(h) {
        Side::Bottom => (f.x, f.y + t - ht),
        Side::Right => (f.x + t - w, f.y),
        _ => (f.x, f.y),
    }
}

pub(super) fn crop(h: HWND, open: (i32, i32), size: (i32, i32)) -> (i32, i32) {
    if side(h).far() { (open.0 - size.0, open.1 - size.1) } else { (0, 0) }
}

pub(super) fn self_sized(h: HWND, f: impl FnOnce()) {
    flagged(h, |v| &mut v.sizing, f);
}

pub(super) fn sizing(h: HWND) -> bool {
    view(h, |v| v.sizing) == Some(true)
}

pub(super) fn resize(h: HWND, f: &Fence) {
    let (w, ht) = size(h, f);
    let (x, y) = origin(h, f, (w, ht));
    self_sized(h, || {
        let _ = unsafe { SetWindowPos(h, None, x, y, w, ht, SWP_NOZORDER | SWP_NOACTIVATE) };
    });
}

pub(super) fn apply(h: HWND) {
    let Some(f) = fence_of(h) else { return };
    resize(h, &f);
    render(h);
}

pub(super) fn persist(h: HWND) {
    let (r, t) = (window_rect(h), bar(h));
    let (x, y) = match side(h) {
        Side::Bottom => (r.left, r.bottom - t),
        Side::Right => (r.right - t, r.top),
        _ => (r.left, r.top),
    };
    let (w, ht) = (r.right - r.left, r.bottom - r.top);
    update(h, |f| {
        (f.x, f.y) = (x, y);
        let (along, across) = if f.vertical { (&mut f.h, &mut f.w) } else { (&mut f.w, &mut f.h) };
        let (len, ext) = if f.vertical { (ht, w) } else { (w, ht) };
        *along = len;
        if !f.rolled && !f.look.auto_height {
            *across = ext;
        }
    });
    orient(h);
    apply(h);
}

pub(super) fn hit(h: HWND, lp: LPARAM) -> u32 {
    let (r, (x, y)) = (window_rect(h), xy(lp));
    let g = geo(h, (r.right - r.left, r.bottom - r.top));
    let (size, p) = g.canonical((x - r.left, y - r.top));
    let f = read(h, |f| (f.locked, f.look.auto_height, f.rolled));
    let folded = f.is_some_and(|(_, _, rolled)| view(h, |v| v.unroll).unwrap_or(if rolled { 0. } else { 1. }) < 1.);
    let z = zone(size, p, scale(h, BORDER), g.bar, folded);
    let z = match f {
        Some((true, _, _)) => z.fixed(),
        Some((_, true, _)) => z.width_only(),
        _ => z,
    };
    match g.side.zone(z) {
        Zone::Client => HTCLIENT,
        Zone::Caption => HTCAPTION,
        Zone::Left => HTLEFT,
        Zone::Right => HTRIGHT,
        Zone::Top => HTTOP,
        Zone::Bottom => HTBOTTOM,
        Zone::TopLeft => HTTOPLEFT,
        Zone::TopRight => HTTOPRIGHT,
        Zone::BottomLeft => HTBOTTOMLEFT,
        Zone::BottomRight => HTBOTTOMRIGHT,
    }
}
