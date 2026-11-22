use super::{cell::PAD, host, layout, modes, read, reload, render, update, view};
use crate::{
    app::with,
    domain::{
        Sort,
        columns::{self, Col, Span},
    },
    i18n,
    layered::{Frame, glyph},
    render::{Canvas, flat, premul},
    win::*,
};
use windows::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::{DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_SINGLELINE, DT_VCENTER, HFONT},
    UI::{
        Input::KeyboardAndMouse::{ReleaseCapture, SetCapture},
        WindowsAndMessaging::{IDC_SIZEWE, LoadCursorW, SetCursor},
    },
};

thread_local! {
    static BEFORE: std::cell::RefCell<Option<crate::domain::Config>> = const { std::cell::RefCell::new(None) };
}

fn hit(h: HWND, (x, y): (i32, i32)) -> Option<(Vec<Span>, i32)> {
    let g = layout::geo_now(h);
    let (x0, y0, x1, y1) = g.head_rect();
    (g.head > 0 && (x0..x1).contains(&x) && (y0..y1).contains(&y)).then(|| (modes::spans(h, g.grid_w()), x - x0))
}

fn edge(h: HWND, p: (i32, i32)) -> Option<(usize, Vec<Span>)> {
    let (spans, x) = hit(h, p)?;
    columns::edge(&spans, x, scale(h, 4)).map(|e| (e, spans))
}

pub(super) fn down(h: HWND, p: (i32, i32)) -> bool {
    if let Some((e, spans)) = edge(h, p) {
        let cols = read(h, |f| f.active().columns()).unwrap_or_default();
        view(h, |v| v.resize = Some((e, cols, spans)));
        BEFORE.with(|b| *b.borrow_mut() = Some(crate::undo::snapshot()));
        unsafe { SetCapture(h) };
        return true;
    }
    let Some((spans, x)) = hit(h, p) else { return false };
    if let Some(col) = columns::at(&spans, x) {
        update(h, |f| {
            let t = f.active_mut();
            (t.sort, t.desc) = columns::click((t.sort, t.desc), col);
        });
        reload(h);
    }
    true
}

pub(super) fn track(h: HWND, (x, _): (i32, i32)) -> bool {
    let Some((e, cols, spans)) = view(h, |v| v.resize.clone()).flatten() else { return false };
    let x = x - layout::geo_now(h).head_rect().0;
    let cols = columns::resize(&cols, &spans, e, x, |v| unscale(h, v));
    with(|a| a.fence_of(h).map(|f| f.active_mut().columns = Some(cols)));
    render(h);
    true
}

pub(super) fn up(h: HWND) -> bool {
    if view(h, |v| v.resize.take()).flatten().is_none() {
        return false;
    }
    let _ = unsafe { ReleaseCapture() };
    crate::app::edit(|_| ());
    if let Some(before) = BEFORE.with(|b| b.borrow_mut().take()) {
        crate::undo::since(before);
    }
    true
}

pub(super) fn cursor(h: HWND) -> bool {
    let sizing = view(h, |v| v.resize.is_some()) == Some(true) || edge(h, super::cursor(h)).is_some();
    if sizing {
        unsafe { SetCursor(LoadCursorW(None, IDC_SIZEWE).ok()) };
    }
    sizing
}

pub(super) fn context(h: HWND, p: (i32, i32)) -> bool {
    if hit(h, p).is_none() {
        return false;
    }
    let shown = read(h, |f| f.active().columns()).unwrap_or_default();
    let Some((m, rtl)) = with(|a| {
        let m = menu();
        for (k, c) in Col::ALL.iter().enumerate().skip(1) {
            item(m, k, a.t(i18n::column(*c)), shown.iter().any(|s| s.col == *c));
        }
        (m, a.rtl())
    }) else {
        return true;
    };
    if let Some(&col) = Col::ALL.get(popup(host::for_menu(h), m, rtl)).filter(|c| **c != Col::Name) {
        update(h, |f| f.active_mut().columns = Some(columns::toggle(&f.active().columns(), col)));
    }
    true
}

pub(super) struct Header<'a> {
    pub spans: &'a [Span],
    pub sort: (Sort, bool),
    pub titles: Vec<String>,
}

pub(super) fn text(
    frame: &Frame,
    (font, glyphs): (HFONT, HFONT),
    (x0, y0, _, y1): (i32, i32, i32, i32),
    hd: &Header,
    s: impl Fn(i32) -> i32,
) {
    let flags = DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS;
    for (&(col, a, b), title) in hd.spans.iter().zip(&hd.titles) {
        let sorted = hd.sort.0 == col.sort();
        let arrow = if sorted { s(16) } else { 0 };
        frame.text(font, title, RECT { left: x0 + a + s(PAD), top: y0, right: x0 + b - s(PAD) - arrow, bottom: y1 }, flags);
        if sorted {
            let g = if hd.sort.1 { glyph::DOWN } else { glyph::UP };
            let r = RECT { left: x0 + b - s(PAD) - arrow, top: y0, right: x0 + b - s(4), bottom: y1 };
            frame.text(glyphs, &g.to_string(), r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        }
    }
}

pub(super) fn shapes(c: &mut Canvas, (x0, y0, x1, y1): (i32, i32, i32, i32), spans: &[Span], ink: u32, s: impl Fn(i32) -> i32) {
    c.rrect((x0, y1 - 1, x1, y1), 0., flat(premul(ink, 40)), (y0, y1), false);
    for &(_, _, b) in spans.iter().filter(|sp| x0 + sp.2 < x1) {
        c.rrect((x0 + b - 1, y0 + s(5), x0 + b, y1 - s(5)), 0., flat(premul(ink, 40)), (y0, y1), false);
    }
}
