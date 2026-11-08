mod hits;
mod paint;

use crate::{domain::search, layered::popup, shell, visibility, win::*};
use hits::Hit;
use std::cell::RefCell;
use windows::{
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
    core::*,
};

const CLASS: PCWSTR = w!("RustDesktopIcons.Search");
const WIDTH: i32 = 560;
const ROW: i32 = 48;

struct State {
    hwnd: HWND,
    query: String,
    all: Vec<Hit>,
    shown: Vec<usize>,
    sel: usize,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|s| s.try_borrow_mut().ok()?.as_mut().map(f))
}

pub fn register_class() {
    register(CLASS, Some(proc));
}

fn rows() -> i32 {
    state(|s| s.shown.len().max(usize::from(!s.query.trim().is_empty())) as i32).unwrap_or(0)
}

fn height(h: HWND) -> i32 {
    scale(h, crate::layered::chrome::TOP + 12 + rows() * ROW)
}

fn refresh(h: HWND) {
    state(|s| {
        let names: Vec<String> = s.all.iter().map(|h| h.name.clone()).collect();
        s.shown = search::rank(&s.query, &names);
        s.sel = s.sel.min(s.shown.len().saturating_sub(1));
    });
    popup::resize(h, (scale(h, WIDTH), height(h)), None);
    paint::render(h);
}

pub fn open(first: Option<char>) {
    if let Some(h) = state(|s| s.hwnd) {
        state(|s| s.query.extend(first));
        refresh(h);
        let _ = unsafe { SetForegroundWindow(h) };
        return;
    }
    let ex = WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW;
    let Some(h) = popup::open(CLASS, ex, WIDTH, |a| a.top + (a.bottom - a.top) / 6) else { return };
    let all = hits::collect();
    STATE.with(|s| *s.borrow_mut() = Some(State { hwnd: h, query: first.into_iter().collect(), all, shown: vec![], sel: 0 }));
    refresh(h);
    popup::show(h);
}

pub fn close() {
    if let Some(h) = state(|s| s.hwnd) {
        let _ = unsafe { DestroyWindow(h) };
    }
}

fn launch(i: usize) {
    let Some(path) = state(|s| s.shown.get(i).map(|&k| s.all[k].path.clone())).flatten() else { return };
    close();
    visibility::end_peek();
    shell::open(&path);
}

fn row_at(h: HWND, (_, y): (i32, i32)) -> Option<usize> {
    let top = scale(h, crate::layered::chrome::TOP + 6);
    (y >= top).then(|| ((y - top) / scale(h, ROW)) as usize).filter(|&i| state(|s| i < s.shown.len()) == Some(true))
}

fn key(h: HWND, vk: u16) -> bool {
    match VIRTUAL_KEY(vk) {
        VK_ESCAPE => close(),
        VK_RETURN => launch(state(|s| s.sel).unwrap_or(0)),
        VK_UP | VK_DOWN => {
            state(|s| s.sel = search::step(s.sel, s.shown.len(), vk == VK_DOWN.0));
            paint::render(h);
        }
        VK_BACK => {
            state(|s| s.query.pop());
            refresh(h);
        }
        _ => return false,
    }
    true
}

unsafe extern "system" fn proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match m {
        WM_KEYDOWN if key(h, wp.0 as u16) => {}
        WM_CHAR if typed(wp).is_some() => {
            state(|s| s.query.extend(typed(wp)));
            refresh(h);
        }
        WM_MOUSEMOVE => {
            if let Some(i) = row_at(h, xy(lp)).filter(|&i| state(|s| s.sel != i) == Some(true)) {
                state(|s| s.sel = i);
                paint::render(h);
            }
        }
        WM_LBUTTONUP => row_at(h, xy(lp)).iter().for_each(|&i| launch(i)),
        WM_ACTIVATE if wp.0 & 0xFFFF == WA_INACTIVE as usize => {
            let _ = unsafe { PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
        WM_DPICHANGED => {
            popup::follow_dpi(h, lp);
            refresh(h);
        }
        WM_CLOSE => close(),
        WM_DESTROY => STATE.with(|s| *s.borrow_mut() = None),
        _ => return unsafe { DefWindowProcW(h, m, wp, lp) },
    }
    LRESULT(0)
}
