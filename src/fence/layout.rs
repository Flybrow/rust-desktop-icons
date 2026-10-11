use super::{BORDER, TITLE, fence_of, flagged, metrics, read, render, update, view, xy};
use crate::{
    domain::{Fence, Zone, anim, grid, zone},
    win::*,
};
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

const MARGIN: i32 = 16;

fn progress(h: HWND, f: &Fence) -> (f32, bool) {
    view(h, |v| (v.unroll, v.opening)).unwrap_or((if f.rolled { 0. } else { 1. }, false))
}

fn fit(h: HWND, f: &Fence, max: i32) -> i32 {
    if !f.look.auto_height {
        return f.h;
    }
    let n = view(h, |v| v.items.len()).unwrap_or(0);
    grid::fit(n, f.w, metrics(h).0, scale(h, TITLE + 4 + 8), (scale(h, Fence::MIN.1), max))
}

pub(super) fn up(h: HWND) -> Option<i32> {
    view(h, |v| v.up).flatten()
}

pub(super) fn full(h: HWND, f: &Fence) -> i32 {
    let (r, m) = (window_rect(h), scale(h, MARGIN));
    let w = work_area(&r);
    fit(h, f, up(h).map_or(w.bottom - r.top, |a| a - w.top) - m)
}

pub(super) fn orient(h: HWND) {
    let Some(f) = fence_of(h).filter(|f| f.rolled) else { return };
    if view(h, |v| v.unroll > 0. || v.up.is_some()) != Some(false) {
        return;
    }
    let (r, bar) = (window_rect(h), scale(h, TITLE));
    let w = work_area(&r);
    if anim::upward(r.top, bar, fit(h, &f, i32::MAX), (w.top, w.bottom), scale(h, MARGIN)) {
        view(h, |v| v.up = Some(r.top + bar));
    }
}

pub(super) fn settle(h: HWND) {
    view(h, |v| {
        if v.unroll == 0. {
            v.up = None;
        }
    });
}

pub(super) fn height(h: HWND, f: &Fence) -> i32 {
    let (p, opening) = progress(h, f);
    anim::height(scale(h, TITLE), full(h, f), p, opening)
}

pub(super) fn self_sized(h: HWND, f: impl FnOnce()) {
    flagged(h, |v| &mut v.sizing, f);
}

pub(super) fn sizing(h: HWND) -> bool {
    view(h, |v| v.sizing) == Some(true)
}

pub(super) fn resize(h: HWND, f: &Fence) {
    let height = height(h, f);
    let (r, flags) = (window_rect(h), SWP_NOZORDER | SWP_NOACTIVATE);
    let (y, flags) = up(h).map_or((0, flags | SWP_NOMOVE), |a| (a - height, flags));
    self_sized(h, || {
        let _ = unsafe { SetWindowPos(h, None, r.left, y, f.w, height, flags) };
    });
}

pub(super) fn apply(h: HWND) {
    let Some(f) = fence_of(h) else { return };
    resize(h, &f);
    render(h);
}

pub(super) fn persist(h: HWND) {
    let (r, up) = (window_rect(h), up(h));
    let top = up.map_or(r.top, |_| r.bottom - scale(h, TITLE));
    if up.is_some() {
        view(h, |v| v.up = Some(r.bottom));
    }
    update(h, |f| {
        (f.x, f.y, f.w) = (r.left, top, r.right - r.left);
        if !f.rolled && !f.look.auto_height {
            f.h = r.bottom - r.top;
        }
    });
}

pub(super) fn hit(h: HWND, lp: LPARAM) -> u32 {
    let (r, (x, y)) = (window_rect(h), xy(lp));
    let f = read(h, |f| (f.locked, f.look.auto_height, f.rolled));
    let folded = f.is_some_and(|(_, _, rolled)| view(h, |v| v.unroll).unwrap_or(if rolled { 0. } else { 1. }) < 1.);
    let z = zone((r.right - r.left, r.bottom - r.top), (x - r.left, y - r.top), scale(h, BORDER), scale(h, TITLE), folded);
    let z = match f {
        Some((true, _, _)) => z.fixed(),
        Some((_, true, _)) => z.width_only(),
        _ => z,
    };
    match z {
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
