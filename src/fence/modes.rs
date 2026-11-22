use super::{cell::Cell, label_font, read};
use crate::{
    app::with,
    domain::{
        Look,
        columns::{self, Mode, Span},
        icons,
    },
    layered::measure,
    win::scale,
};
use windows::Win32::{Foundation::HWND, Graphics::Gdi::DT_SINGLELINE};

pub(super) const HEAD: i32 = 26;
const TILE: i32 = 260;

pub(super) fn mode(h: HWND) -> Mode {
    read(h, |f| f.mode()).unwrap_or_default()
}

pub(super) fn metrics(h: HWND) -> (i32, i32) {
    let Some((look, mode)) = read(h, |f| (f.look.clone(), f.mode())) else { return (scale(h, 80), scale(h, 32)) };
    metrics_of(h, &look, mode)
}

pub(super) fn metrics_of(h: HWND, look: &Look, mode: Mode) -> (i32, i32) {
    let s = |v| scale(h, v);
    let (row, icon) = match mode {
        Mode::Icons => return (s(icons::cell(look.icon, look.labels.then_some(look.label_size), look.spacing)), s(look.icon)),
        Mode::List => (12, look.icon.min(32)),
        Mode::Details => (8, 16),
        Mode::Content => (16, 32),
        Mode::Tiles => (16, 48),
    };
    (s(icon + row), s(icon))
}

pub(super) fn head(h: HWND) -> i32 {
    if mode(h) == Mode::Details { scale(h, HEAD) } else { 0 }
}

pub(super) fn spans(h: HWND, gw: i32) -> Vec<Span> {
    let cols = read(h, |f| f.active().columns()).unwrap_or_default();
    columns::spans(&cols, gw, |v| scale(h, v))
}

thread_local! {
    static LINES: std::cell::RefCell<Vec<(isize, i32)>> = const { std::cell::RefCell::new(vec![]) };
}

fn line(font: windows::Win32::Graphics::Gdi::HFONT) -> i32 {
    let key = font.0 as isize;
    if let Some(l) = LINES.with(|c| c.borrow().iter().find(|e| e.0 == key).map(|e| e.1)) {
        return l;
    }
    let l = measure(font, &[b'A' as u16, b'g' as u16], 1000, DT_SINGLELINE);
    LINES.with(|c| c.borrow_mut().push((key, l)));
    l
}

pub(super) fn shape(h: HWND, gw: i32) -> Cell {
    let (cell, mode) = (metrics(h).0, mode(h));
    let line = with(|a| line(label_font(a, h))).unwrap_or(0);
    let w = match mode {
        Mode::Icons => cell,
        Mode::Tiles => scale(h, TILE).min(gw),
        _ => gw,
    };
    let name_w = if mode == Mode::Details { spans(h, gw)[0].2 } else { w };
    Cell { w, h: cell, mode, name_w, line }
}
