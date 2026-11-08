mod act;
mod layout;
mod paint;
mod rows;

use crate::{
    app::with,
    layered::{chrome::inside, popup},
    win::*,
};
use crate::{domain::hotkey::Slot, visibility::hotkeys};
use windows::{
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::VK_ESCAPE, WindowsAndMessaging::*},
    },
    core::*,
};

const CLASS: PCWSTR = w!("RustDesktopIcons.Settings");
const SIDE_TOP: i32 = 12;

pub struct Panel {
    pub hwnd: HWND,
    pub hover: Option<usize>,
    pub page_hover: Option<usize>,
    pub page: usize,
    pub close_hot: bool,
    pub scroll: i32,
    pub capture: Option<Slot>,
}

pub fn register_class() {
    register(CLASS, Some(proc));
}

fn hwnd() -> Option<HWND> {
    with(|a| a.panel.as_ref().map(|p| p.hwnd)).flatten()
}

fn height(h: HWND) -> i32 {
    let area = work_area(&window_rect(h));
    let tallest = scale(h, rows::TOP + 16) + scale(h, with(|a| rows::tallest(a)).unwrap_or(0));
    tallest.min(area.bottom - area.top - scale(h, 60))
}

pub fn refresh() {
    let Some(h) = hwnd() else { return };
    let r = window_rect(h);
    popup::resize(h, (r.right - r.left, height(h)), None);
    paint::render(h);
}

pub fn open() {
    crate::visibility::end_peek();
    if let Some(h) = hwnd() {
        let _ = unsafe { SetForegroundWindow(h) };
        return refresh();
    }
    let Some(h) = popup::open(CLASS, WS_EX_LAYERED | WS_EX_APPWINDOW, rows::WIDTH, |a| a.top + sys_scale(60)) else { return };
    with(|a| a.panel = Some(Panel { hwnd: h, hover: None, page_hover: None, page: 0, close_hot: false, scroll: 0, capture: None }));
    refresh();
    popup::show(h);
}

fn close() {
    if let Some(h) = hwnd() {
        unsafe { DestroyWindow(h).ok() };
    }
}

fn content(h: HWND) -> i32 {
    let total = with(|a| rows::total(&rows::build(a, a.panel.as_ref().map_or(0, |p| p.page)))).unwrap_or(0);
    scale(h, rows::TOP + 16) + scale(h, total)
}

pub(super) fn mid(h: HWND) -> i32 {
    let (w, side) = (client_rect(h).right, scale(h, rows::SIDE));
    side + (w - side) / 2 + scale(h, 20)
}

fn unscale(h: HWND, v: i32) -> i32 {
    v * 96 / scale(h, 96)
}

fn page_at(h: HWND, (x, y): (i32, i32)) -> Option<usize> {
    (x < scale(h, rows::SIDE)).then(|| rows::page_at(unscale(h, y - scale(h, rows::TOP + SIDE_TOP)))).flatten()
}

fn row_hit(h: HWND, (x, y): (i32, i32)) -> Option<(usize, i32)> {
    let top = scale(h, rows::TOP);
    if y < top || x < scale(h, rows::SIDE) {
        return None;
    }
    with(|a| {
        let p = a.panel.as_ref()?;
        rows::hit(&rows::build(a, p.page), unscale(h, y - top + p.scroll))
    })
    .flatten()
}

fn row_at(h: HWND, p: (i32, i32)) -> Option<usize> {
    row_hit(h, p).map(|(i, _)| i)
}

fn track(h: HWND, p: (i32, i32)) {
    track_leave(h);
    let (row, page, hot) = (row_at(h, p), page_at(h, p), inside(&popup::close_rect(h), p));
    let changed = with(|a| {
        let panel = a.panel.as_mut()?;
        let changed = (panel.hover, panel.page_hover, panel.close_hot) != (row, page, hot);
        (panel.hover, panel.page_hover, panel.close_hot) = (row, page, hot);
        Some(changed)
    });
    if changed.flatten() == Some(true) {
        paint::render(h);
    }
}

fn capturing() -> Option<Slot> {
    with(|a| a.panel.as_ref().and_then(|p| p.capture)).flatten()
}

fn stop_capture() {
    with(|a| a.panel.as_mut().map(|p| p.capture = None));
    hotkeys::register();
    refresh();
}

fn capture(vk: u16) {
    if vk == VK_ESCAPE.0 {
        return stop_capture();
    }
    if let (Some(s), Some(k)) = (capturing(), hotkeys::captured(vk))
        && hotkeys::set(s, k)
    {
        stop_capture();
    }
}

fn select(h: HWND, page: usize) {
    with(|a| a.panel.as_mut().map(|p| (p.page, p.scroll, p.hover) = (page, 0, None)));
    paint::render(h);
}

unsafe extern "system" fn proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match m {
        WM_NCHITTEST => return popup::hit(h, lp),
        WM_MOUSEMOVE => track(h, xy(lp)),
        WM_MOUSELEAVE => {
            with(|a| a.panel.as_mut().map(|p| (p.hover, p.page_hover, p.close_hot) = (None, None, false)));
            paint::render(h);
        }
        WM_LBUTTONUP => {
            let p = xy(lp);
            if inside(&popup::close_rect(h), p) {
                close();
            } else if let Some(page) = page_at(h, p) {
                select(h, page);
            } else if let Some((i, offset)) = row_hit(h, p) {
                act::click(h, i, offset);
            }
        }
        WM_MOUSEWHEEL => {
            let step = wheel(wp) * scale(h, 60) / 120;
            let max = (content(h) - client_rect(h).bottom).max(0);
            with(|a| a.panel.as_mut().map(|p| p.scroll = (p.scroll - step).clamp(0, max)));
            paint::render(h);
        }
        WM_KEYDOWN | WM_SYSKEYDOWN if capturing().is_some() => capture(wp.0 as u16),
        WM_CHAR | WM_SYSCHAR if capturing().is_some() => {}
        WM_KEYDOWN if wp.0 as u16 == VK_ESCAPE.0 => close(),
        WM_DPICHANGED => {
            popup::follow_dpi(h, lp);
            refresh();
        }
        WM_ACTIVATE if wp.0 & 0xFFFF == WA_INACTIVE as usize && capturing().is_some() => stop_capture(),
        WM_DESTROY => {
            let held = capturing().is_some();
            with(|a| a.panel = None);
            if held {
                hotkeys::register();
            }
        }
        _ => return unsafe { DefWindowProcW(h, m, wp, lp) },
    }
    LRESULT(0)
}
