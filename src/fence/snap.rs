use super::{group, guides, items, layout, metrics, read, view};
use crate::{
    app::with,
    domain::{Fence, snap},
    win::*,
};
use windows::Win32::{
    Foundation::*,
    UI::{
        Input::KeyboardAndMouse::VK_MENU,
        WindowsAndMessaging::{
            WMSZ_BOTTOM, WMSZ_BOTTOMLEFT, WMSZ_BOTTOMRIGHT, WMSZ_LEFT, WMSZ_RIGHT, WMSZ_TOP, WMSZ_TOPLEFT, WMSZ_TOPRIGHT,
        },
    },
};

const DIST: i32 = 16;
const GAP: i32 = 12;

fn get(lp: LPARAM) -> snap::Rect {
    let r = unsafe { &*(lp.0 as *const RECT) };
    [r.left, r.top, r.right, r.bottom]
}

fn set(lp: LPARAM, [left, top, right, bottom]: snap::Rect) {
    unsafe { *(lp.0 as *mut RECT) = RECT { left, top, right, bottom } };
}

fn area(r: snap::Rect) -> snap::Rect {
    let a = work_area(&RECT { left: r[0], top: r[1], right: r[2], bottom: r[3] });
    [a.left, a.top, a.right, a.bottom]
}

fn others(h: HWND) -> Vec<snap::Rect> {
    let mates = group::mates(h);
    items::windows()
        .into_iter()
        .filter(|w| *w != h && !mates.contains(w))
        .map(window_rect)
        .map(|r| [r.left, r.top, r.right, r.bottom])
        .collect()
}

fn free() -> bool {
    held(VK_MENU)
}

fn anchor(h: HWND, at: (i32, i32)) -> ([i32; 4], (i32, i32)) {
    let r = window_rect(h);
    ([r.left, r.top, r.right, r.bottom], at)
}

pub(super) fn press(h: HWND, at: (i32, i32), f: impl FnOnce() -> LRESULT) -> LRESULT {
    let start = anchor(h, at);
    view(h, |v| v.drag = Some(start));
    let out = f();
    end(h);
    out
}

pub(super) fn begin(h: HWND) {
    let start = anchor(h, cursor_pos());
    view(h, |v| v.drag = v.drag.or(Some(start)));
    group::grab(h);
}

pub(super) fn end(h: HWND) {
    view(h, |v| v.drag = None);
    guides::hide();
    group::release();
}

pub(super) fn moving(h: HWND, lp: LPARAM) {
    let drag = with(|a| a.view(h).and_then(|v| v.drag)).flatten();
    let r = drag.map_or(get(lp), |(start, c0)| snap::follow(start, c0, cursor_pos()));
    if free() {
        set(lp, r);
        group::follow(r[0], r[1]);
        return guides::hide();
    }
    let (out, lines) = snap::moving(r, area(r), &others(h), scale(h, DIST), scale(h, GAP));
    set(lp, out);
    group::follow(out[0], out[1]);
    guides::show(h, &lines);
}

fn grid(h: HWND, r: snap::Rect, sides: [bool; 4], a: snap::Rect) -> (Option<i32>, Option<i32>) {
    let Some((vertical, auto)) = read(h, |f| (f.vertical, f.look.auto_height)) else { return (None, None) };
    let (cell, open) = (metrics(h).0, view(h, |v| v.unroll >= 1.) == Some(true));
    let room = |i: usize| if sides[i] { r[i + 2] - a[i] } else { a[i + 2] - r[i] };
    let along = |i: usize| Some(snap::grid_len(r[i + 2] - r[i], 0, cell, scale(h, Fence::MIN.0), room(i)));
    let across =
        |i: usize| (open && !auto).then(|| snap::grid_len(r[i + 2] - r[i], layout::chrome(h, vertical), cell, layout::bar(h), room(i)));
    if vertical { (across(0), along(1)) } else { (along(0), across(1)) }
}

pub(super) fn sizing(h: HWND, wp: WPARAM, lp: LPARAM) {
    if free() {
        return guides::hide();
    }
    let e = wp.0 as u32;
    let sides = [
        matches!(e, WMSZ_LEFT | WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT),
        matches!(e, WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT),
        matches!(e, WMSZ_RIGHT | WMSZ_TOPRIGHT | WMSZ_BOTTOMRIGHT),
        matches!(e, WMSZ_BOTTOM | WMSZ_BOTTOMLEFT | WMSZ_BOTTOMRIGHT),
    ];
    let a = area(get(lp));
    let r = snap::sizing(get(lp), a, scale(h, DIST), sides);
    let r = snap::resize(r, sides, grid(h, r, sides, a));
    set(lp, r);
    guides::show(h, &snap::guides(r, &others(h), scale(h, GAP)));
}
