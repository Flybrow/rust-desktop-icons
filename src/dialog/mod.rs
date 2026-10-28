mod layout;
mod paint;

use crate::{
    app::with,
    layered::chrome::{self, inside},
    win::*,
};
use layout::{Layout, Metrics, hit, layout};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::ScreenToClient,
        UI::{Input::KeyboardAndMouse::VK_ESCAPE, WindowsAndMessaging::*},
    },
    core::*,
};

const CLASS: PCWSTR = w!("RustDesktopIcons.Dialog");
const WIDTH: i32 = 460;
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

fn current(h: HWND) -> Layout {
    let m = metrics(h);
    let (font, spec) = with(|a| {
        (a.font(scale(h, 13), 400), a.dialog.as_ref().map(|d| (d.spec.body.clone(), d.spec.progress.is_some(), d.spec.buttons.len())))
    })
    .unwrap_or_default();
    let (body, progress, buttons) = spec.unwrap_or_default();
    layout(&m, paint::measure(font, &body, m.width - 2 * m.pad), progress, buttons)
}

fn refresh(h: HWND) {
    let l = current(h);
    let r = window_rect(h);
    if r.bottom - r.top != l.height {
        let area = work_area(&r);
        let y = area.top + (area.bottom - area.top - l.height) / 2;
        let _ = unsafe { SetWindowPos(h, None, r.left, y, r.right - r.left, l.height, SWP_NOZORDER | SWP_NOACTIVATE) };
    }
    paint::render(h, &l);
}

pub fn show(spec: Spec) {
    if let Some(h) = hwnd() {
        with(|a| a.dialog.as_mut().map(|d| (d.spec, d.scroll, d.hover) = (spec, 0, None)));
        let _ = unsafe { KillTimer(Some(h), ANIM) };
        refresh(h);
        let _ = unsafe { SetForegroundWindow(h) };
        return;
    }
    let s = sys_scale;
    let (cx, cy) = cursor_pos();
    let area = work_area(&RECT { left: cx, top: cy, right: cx + 1, bottom: cy + 1 });
    let w = s(WIDTH);
    let x = area.left + (area.right - area.left - w) / 2;
    let ex = WS_EX_LAYERED | WS_EX_APPWINDOW | WS_EX_TOPMOST;
    let Ok(h) = (unsafe {
        CreateWindowExW(ex, CLASS, w!("Rust Desktop Icons"), WS_POPUP, x, area.top + s(120), w, s(200), None, None, Some(inst()), None)
    }) else {
        return;
    };
    with(|a| a.dialog = Some(Dialog { hwnd: h, spec, hover: None, close_hot: false, scroll: 0, phase: 0 }));
    refresh(h);
    unsafe {
        let _ = ShowWindow(h, SW_SHOW);
        let _ = SetForegroundWindow(h);
    }
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
    let (buttons, close) = (current(h).buttons, chrome::close_rect(client_rect(h).right, |v| scale(h, v)));
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
    let close = chrome::close_rect(client_rect(h).right, |v| scale(h, v));
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
        WM_NCHITTEST => {
            let (x, y) = xy(lp);
            let mut p = POINT { x, y };
            let _ = unsafe { ScreenToClient(h, &mut p) };
            let close = chrome::close_rect(client_rect(h).right, |v| scale(h, v));
            let caption = p.y < scale(h, chrome::TOP) && !inside(&close, (p.x, p.y));
            return LRESULT(if caption { HTCAPTION } else { HTCLIENT } as isize);
        }
        WM_MOUSEMOVE => track(h, xy(lp)),
        WM_MOUSELEAVE => {
            with(|a| a.dialog.as_mut().map(|d| (d.hover, d.close_hot) = (None, false)));
            paint::render(h, &current(h));
        }
        WM_LBUTTONUP => click(h, xy(lp)),
        WM_MOUSEWHEEL => {
            let l = current(h);
            let step = (wp.0 >> 16) as i16 as i32 * scale(h, 60) / 120;
            let font = with(|a| a.font(scale(h, 13), 400)).unwrap_or_default();
            let full = with(|a| a.dialog.as_ref().map(|d| d.spec.body.clone())).flatten().unwrap_or_default();
            let max = (paint::measure(font, &full, l.body.2 - l.body.0) - (l.body.3 - l.body.1)).max(0);
            with(|a| a.dialog.as_mut().map(|d| d.scroll = (d.scroll - step).clamp(0, max)));
            paint::render(h, &l);
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
        WM_DPICHANGED => refresh(h),
        WM_DESTROY => {
            with(|a| a.dialog = None);
        }
        _ => return unsafe { DefWindowProcW(h, m, wp, lp) },
    }
    LRESULT(0)
}
