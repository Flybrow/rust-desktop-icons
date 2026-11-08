mod layout;
mod notes;
mod paint;

use crate::update::notes::Block;
use crate::{
    app::{App, with},
    layered::{
        chrome::{self, inside},
        popup,
    },
    win::*,
};
use layout::{Layout, Metrics, hit, layout};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::HFONT,
        UI::{
            Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_NEXT, VK_PRIOR, VK_UP},
            WindowsAndMessaging::*,
        },
    },
    core::*,
};

const CLASS: PCWSTR = w!("RustDesktopIcons.Dialog");
const WIDTH: i32 = 600;
const ANIM: usize = 1;

pub struct Button {
    pub label: String,
    pub act: fn(),
}

pub struct Progress {
    pub value: Option<f32>,
    pub text: String,
}

pub struct Spec {
    pub title: String,
    pub body: String,
    pub buttons: Vec<Button>,
    pub progress: Option<Progress>,
    pub dismiss: fn(),
    pub notes: Vec<Block>,
}

pub struct Dialog {
    pub hwnd: HWND,
    pub spec: Spec,
    pub hover: Option<usize>,
    pub close_hot: bool,
    pub scroll: i32,
    pub phase: u32,
}

pub fn register_class() {
    register(CLASS, Some(proc));
}

fn hwnd() -> Option<HWND> {
    with(|a| a.dialog.as_ref().map(|d| d.hwnd)).flatten()
}

pub fn close() {
    if let Some(h) = hwnd() {
        unsafe { DestroyWindow(h).ok() };
    }
}

fn metrics(h: HWND) -> Metrics {
    let s = |v| scale(h, v);
    let area = work_area(&window_rect(h));
    Metrics {
        width: s(WIDTH),
        top: s(chrome::TOP),
        pad: s(22),
        gap: s(16),
        line: s(20),
        bar: s(8),
        button: (s(120), s(36)),
        max_body: (area.bottom - area.top) * 3 / 5,
    }
}

fn fonts(a: &mut App, h: HWND) -> (HFONT, notes::Fonts) {
    let s = |v| scale(h, v);
    let f = notes::Fonts { section: a.font(s(12), 600), title: a.font(s(13), 600), desc: a.font(s(13), 400) };
    (f.desc, f)
}

fn text(h: HWND, width: i32) -> (i32, Vec<notes::Line>) {
    let Some((font, f, body, blocks)) = with(|a| {
        let (font, f) = fonts(a, h);
        a.dialog.as_ref().map(|d| (font, f, d.spec.body.clone(), d.spec.notes.clone()))
    })
    .flatten() else {
        return (0, vec![]);
    };
    let top = if body.is_empty() { 0 } else { paint::measure(font, &body, width) + scale(h, 14) };
    let (mut lines, height) = notes::place(&blocks, f, width, |v| scale(h, v));
    lines.iter_mut().for_each(|l| (l.rect.1, l.rect.3) = (l.rect.1 + top, l.rect.3 + top));
    (top + height, lines)
}

fn current(h: HWND) -> Layout {
    let m = metrics(h);
    let font = with(|a| fonts(a, h).0).unwrap_or_default();
    let (progress, buttons) =
        with(|a| a.dialog.as_ref().map(|d| (d.spec.progress.is_some(), d.spec.buttons.len()))).flatten().unwrap_or_default();
    let line = paint::measure(font, "X", m.width).max(1);
    let m = Metrics { max_body: (m.max_body / line).max(1) * line, ..m };
    layout(&m, text(h, m.width - 2 * m.pad).0, progress, buttons)
}

fn scroll_by(h: HWND, delta: i32) {
    let l = current(h);
    let max = (l.text - (l.body.3 - l.body.1)).max(0);
    with(|a| a.dialog.as_mut().map(|d| d.scroll = (d.scroll - delta).clamp(0, max)));
    paint::render(h, &l);
}

fn refresh(h: HWND) {
    let l = current(h);
    let r = window_rect(h);
    if r.bottom - r.top != l.height {
        let area = work_area(&r);
        popup::resize(h, (r.right - r.left, l.height), Some(area.top + (area.bottom - area.top - l.height) / 2));
    }
    paint::render(h, &l);
}

pub fn show(spec: Spec) {
    crate::visibility::end_peek();
    if let Some(h) = hwnd() {
        with(|a| a.dialog.as_mut().map(|d| (d.spec, d.scroll, d.hover) = (spec, 0, None)));
        let _ = unsafe { KillTimer(Some(h), ANIM) };
        refresh(h);
        let _ = unsafe { SetForegroundWindow(h) };
        return;
    }
    let ex = WS_EX_LAYERED | WS_EX_APPWINDOW | WS_EX_TOPMOST;
    let Some(h) = popup::open(CLASS, ex, WIDTH, |a| a.top + sys_scale(120)) else { return };
    with(|a| a.dialog = Some(Dialog { hwnd: h, spec, hover: None, close_hot: false, scroll: 0, phase: 0 }));
    refresh(h);
    popup::show(h);
}

pub fn progress(value: Option<f32>, text: String) {
    let Some(h) = hwnd() else { return };
    with(|a| {
        a.dialog.as_mut().map(|d| {
            d.spec.buttons.clear();
            d.spec.progress = Some(Progress { value, text });
        })
    });
    unsafe {
        if value.is_none() {
            SetTimer(Some(h), ANIM, 30, None);
        } else {
            let _ = KillTimer(Some(h), ANIM);
        }
    }
    refresh(h);
}

fn track(h: HWND, p: (i32, i32)) {
    track_leave(h);
    let (buttons, close) = (current(h).buttons, popup::close_rect(h));
    let (over, hot) = (hit(&buttons, p), inside(&close, p));
    let changed = with(|a| {
        let d = a.dialog.as_mut()?;
        let changed = d.hover != over || d.close_hot != hot;
        (d.hover, d.close_hot) = (over, hot);
        Some(changed)
    });
    if changed.flatten() == Some(true) {
        paint::render(h, &current(h));
    }
}

fn click(h: HWND, p: (i32, i32)) {
    let close = popup::close_rect(h);
    let act = if inside(&close, p) {
        with(|a| a.dialog.as_ref().map(|d| d.spec.dismiss)).flatten()
    } else {
        let i = hit(&current(h).buttons, p);
        with(|a| Some(a.dialog.as_ref()?.spec.buttons.get(i?)?.act)).flatten()
    };
    if let Some(act) = act {
        act();
    }
}

unsafe extern "system" fn proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match m {
        WM_NCHITTEST => return popup::hit(h, lp),
        WM_MOUSEMOVE => track(h, xy(lp)),
        WM_MOUSELEAVE => {
            with(|a| a.dialog.as_mut().map(|d| (d.hover, d.close_hot) = (None, false)));
            paint::render(h, &current(h));
        }
        WM_LBUTTONUP => click(h, xy(lp)),
        WM_MOUSEWHEEL => scroll_by(h, wheel(wp) * scale(h, 60) / 120),
        WM_KEYDOWN if matches!(VIRTUAL_KEY(wp.0 as u16), VK_DOWN | VK_UP | VK_NEXT | VK_PRIOR | VK_HOME | VK_END) => {
            let page = scale(h, 300);
            scroll_by(
                h,
                match VIRTUAL_KEY(wp.0 as u16) {
                    VK_DOWN => -scale(h, 40),
                    VK_UP => scale(h, 40),
                    VK_NEXT => -page,
                    VK_PRIOR => page,
                    VK_HOME => i32::MAX / 2,
                    _ => -i32::MAX / 2,
                },
            );
        }
        WM_TIMER if wp.0 == ANIM => {
            with(|a| a.dialog.as_mut().map(|d| d.phase = d.phase.wrapping_add(1)));
            paint::render(h, &current(h));
        }
        WM_KEYDOWN if wp.0 as u16 == VK_ESCAPE.0 => {
            if let Some(f) = with(|a| a.dialog.as_ref().map(|d| d.spec.dismiss)).flatten() {
                f();
            }
        }
        WM_DPICHANGED => {
            popup::follow_dpi(h, lp);
            refresh(h);
        }
        WM_EXITSIZEMOVE | WM_DISPLAYCHANGE => refresh(h),
        WM_DESTROY => {
            with(|a| a.dialog = None);
        }
        _ => return unsafe { DefWindowProcW(h, m, wp, lp) },
    }
    LRESULT(0)
}
