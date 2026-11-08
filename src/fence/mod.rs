mod actions;
mod anim;
mod arrange;
mod cell;
mod drop;
mod edit;
mod ghost;
mod guides;
mod header;
mod host;
mod items;
mod keys;
mod layout;
mod loader;
mod menu;
mod native;
mod paint;
mod peek;
mod select;
mod snap;
mod system;
mod tabs;
mod target;
mod window;

use crate::{
    app::{App, View, with},
    domain::{Fence, icons},
    shell,
    win::*,
};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::{FW_NORMAL, FW_SEMIBOLD, HFONT, ScreenToClient},
        UI::WindowsAndMessaging::*,
    },
    core::*,
};

pub use edit::key;
pub use items::{reload, reload_all, windows as hwnds};
pub use paint::render;
pub use tabs::bind;
use window::top;
pub use window::{create, footprints, raise};

pub const CLASS: PCWSTR = w!("RustDesktopIcons.Fence");
pub const WM_CHANGED: u32 = WM_APP + 10;
const TITLE: i32 = 34;
const BORDER: i32 = 6;
const MK_CONTROL: usize = 0x0008;
const TABS: usize = 64;
const RELOAD: usize = 1;
const PEEK: usize = 2;
const ANIM: usize = 3;
const PREPARE: usize = 5;

fn title_font(a: &mut App, h: HWND) -> HFONT {
    a.font(scale(h, 14), FW_SEMIBOLD.0)
}

fn label_font(a: &mut App, h: HWND) -> HFONT {
    a.font(scale(h, 12), FW_NORMAL.0)
}

fn to_client(h: HWND, (x, y): (i32, i32)) -> (i32, i32) {
    let mut p = POINT { x, y };
    let _ = unsafe { ScreenToClient(h, &mut p) };
    (p.x, p.y)
}

fn cursor(h: HWND) -> (i32, i32) {
    to_client(h, cursor_pos())
}

fn view<R>(h: HWND, f: impl FnOnce(&mut View) -> R) -> Option<R> {
    with(|a| a.view(h).map(f)).flatten()
}

fn flagged(h: HWND, flag: fn(&mut View) -> &mut bool, f: impl FnOnce()) {
    view(h, |v| *flag(v) = true);
    f();
    view(h, |v| *flag(v) = false);
}

fn read<R>(h: HWND, f: impl FnOnce(&Fence) -> R) -> Option<R> {
    with(|a| a.fence_of(h).map(|x| f(x))).flatten()
}

fn fence_of(h: HWND) -> Option<Fence> {
    with(|a| a.fence_of(h).cloned()).flatten()
}

fn update(h: HWND, f: impl FnOnce(&mut Fence)) {
    crate::app::edit(|a| a.fence_of(h).map(f));
    layout::apply(h);
}

fn restyle(h: HWND) {
    layout::apply(h);
    anim::start(h);
}

pub fn register_classes() {
    guides::register_class();
    host::register_class();
}

pub fn restyle_all() {
    items::windows().into_iter().for_each(restyle);
}

fn metrics(h: HWND) -> (i32, i32) {
    let (px, labels) = read(h, |f| (f.look.icon, f.look.labels)).unwrap_or((icons::DEFAULT, true));
    (scale(h, icons::cell(px, labels)), scale(h, px))
}

pub unsafe extern "system" fn proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match m {
        WM_NCHITTEST => return LRESULT(layout::hit(h, lp) as isize),
        WM_MOUSEACTIVATE => return LRESULT(if (lp.0 & 0xFFFF) as u32 == HTCLIENT { MA_ACTIVATE } else { MA_NOACTIVATE } as isize),
        WM_KEYDOWN if keys::handle(h, wp.0 as u16) => {}
        WM_SIZE if !layout::sizing(h) => render(h),
        WM_WINDOWPOSCHANGING if !top() && !crate::visibility::peeking() => keep_bottom(lp),
        WM_GETMINMAXINFO => {
            let i = unsafe { &mut *(lp.0 as *mut MINMAXINFO) };
            let (len, bar) = (scale(h, Fence::MIN.0), scale(h, TITLE));
            i.ptMinTrackSize = if read(h, |f| f.vertical) == Some(true) { POINT { x: bar, y: len } } else { POINT { x: len, y: bar } };
        }
        WM_ENTERSIZEMOVE => snap::begin(h),
        WM_EXITSIZEMOVE => {
            snap::end(h);
            layout::persist(h);
        }
        WM_DPICHANGED => crate::layered::popup::follow_dpi(h, lp),
        WM_MOVING => {
            if read(h, |f| f.locked) == Some(true) {
                unsafe { *(lp.0 as *mut RECT) = window_rect(h) };
            } else {
                snap::moving(h, lp);
            }
            return LRESULT(1);
        }
        WM_SIZING => {
            snap::sizing(h, wp, lp);
            return LRESULT(1);
        }
        WM_NCLBUTTONDOWN if wp.0 as u32 == HTCAPTION && tabs::click(h, xy(lp)) => {}
        WM_NCLBUTTONDOWN => return snap::press(h, xy(lp), || unsafe { DefWindowProcW(h, m, wp, lp) }),
        WM_NCLBUTTONDBLCLK if wp.0 as u32 == HTCAPTION => peek::toggle(h),
        WM_NCMOUSEMOVE => {
            peek::enter(h);
            tabs::hover(h, xy(lp));
        }
        WM_NCRBUTTONUP if wp.0 as u32 == HTCAPTION => peek::keep_open(h, || menu::context(h)),
        WM_LBUTTONDBLCLK => items::item_at(h, xy(lp)).iter().for_each(|p| shell::open(p)),
        WM_LBUTTONDOWN => select::down(h, xy(lp), wp.0 & MK_CONTROL != 0),
        WM_LBUTTONUP | WM_CAPTURECHANGED => select::up(h),
        WM_CONTEXTMENU => peek::keep_open(h, || menu::context(h)),
        WM_MOUSEMOVE => {
            track_leave(h);
            peek::enter(h);
            if !select::track(h, xy(lp)) {
                items::hover(h, items::index_at(h, xy(lp)));
            }
        }
        WM_MOUSELEAVE => items::hover(h, None),
        WM_MOUSEWHEEL if wp.0 & MK_CONTROL != 0 => {
            if let Some(f) = fence_of(h) {
                items::set_icon(h, icons::step(f.look.icon, wheel(wp) > 0));
            }
        }
        WM_MOUSEWHEEL => {
            let step = wheel(wp) * metrics(h).0 / 120;
            view(h, |v| v.scroll -= step);
            render(h);
        }
        WM_DESTROY => {
            target::revoke(h);
            loader::discard(h);
        }
        m if (WM_CHANGED..WM_CHANGED + TABS as u32).contains(&m) => {
            let i = (m - WM_CHANGED) as usize;
            view(h, |v| (!v.dirty.contains(&i)).then(|| v.dirty.push(i)));
            unsafe { SetTimer(Some(h), RELOAD, 150, None) };
        }
        loader::WM_LOADED => loader::receive(h, lp),
        WM_TIMER if wp.0 == RELOAD => {
            let _ = unsafe { KillTimer(Some(h), RELOAD) };
            items::refresh(h);
        }
        WM_TIMER if wp.0 == PEEK => peek::check(h),
        WM_TIMER if wp.0 == ANIM => anim::tick(h),
        WM_TIMER if wp.0 == PREPARE => paint::prepare(h),
        WM_COMMAND if (wp.0 >> 16) as u32 == EN_KILLFOCUS => edit::finish(h, true),
        _ => return unsafe { DefWindowProcW(h, m, wp, lp) },
    }
    LRESULT(0)
}
