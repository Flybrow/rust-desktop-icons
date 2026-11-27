mod actions;
mod anim;
mod arrange;
mod browse;
mod cell;
mod columns;
mod desks;
mod detail;
mod drop;
mod edit;
mod field;
mod filter;
mod ghost;
mod glassbar;
mod group;
mod guides;
mod header;
mod host;
mod items;
mod keys;
mod layout;
mod loader;
mod menu;
mod modes;
mod moveto;
mod native;
mod nav;
mod owner;
mod pacer;
mod paint;
mod peek;
mod select;
mod snap;
mod system;
mod tabs;
mod target;
mod titlebar;
mod wallpaper;
mod window;

use crate::{
    app::{App, View, with},
    domain::{Fence, TABS, icons},
    win::*,
};
use modes::{metrics, shape};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::{FW_NORMAL, FW_SEMIBOLD, HFONT, ScreenToClient},
        UI::WindowsAndMessaging::*,
    },
    core::*,
};

pub use actions::evacuate;
pub use edit::key;
pub use items::{reload, reload_all, windows as hwnds};
pub use owner::{ADOPT, adopt, bind as bind_owner};
pub use paint::render;
pub use tabs::bind;
pub use wallpaper::changed as wallpaper_changed;
use window::top;
pub use window::{create, footprints, raise};

pub const CLASS: PCWSTR = w!("RustDesktopIcons.Fence");
pub const WM_CHANGED: u32 = WM_APP + 10;
const TITLE: i32 = 34;
const GLASS_TITLE: i32 = 40;
const BORDER: i32 = 6;
const MK_CONTROL: usize = 0x0008;
const RELOAD: usize = 1;
const PEEK: usize = 2;
const PREPARE: usize = 5;
const CARET: usize = 6;

fn title_font(a: &mut App, h: HWND) -> HFONT {
    bar_font(a, h, FW_SEMIBOLD.0)
}

fn bar_font(a: &mut App, h: HWND, weight: u32) -> HFONT {
    let px = a.fence_of(h).map_or(14, |f| f.look.title_size.title_px());
    a.font(scale(h, px), weight)
}

fn label_font(a: &mut App, h: HWND) -> HFONT {
    let px = a.fence_of(h).map_or(12, |f| f.look.label_size.px());
    a.font(scale(h, px), FW_NORMAL.0)
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

fn update_tab(h: HWND, tab: u64, f: impl FnOnce(&mut crate::domain::Tab)) {
    update(h, |x| {
        if let Some(t) = x.tabs.iter_mut().find(|t| t.id == tab) {
            f(t)
        }
    });
}

fn window(fence: u64) -> Option<HWND> {
    with(|a| a.views.iter().find(|v| v.id == fence).map(|v| v.hwnd)).flatten()
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

pub unsafe extern "system" fn proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match m {
        WM_NCHITTEST => return LRESULT(layout::hit(h, lp) as isize),
        WM_MOUSEACTIVATE => return LRESULT(if (lp.0 & 0xFFFF) as u32 == HTCLIENT { MA_ACTIVATE } else { MA_NOACTIVATE } as isize),
        WM_KEYDOWN if crate::undo::shortcut(wp.0 as u16) => crate::undo::undo(),
        WM_CHAR if filter::typed(h, wp) => {}
        WM_SETFOCUS => filter::focus(h),
        WM_KILLFOCUS => {
            filter::left(h);
            browse::left(h, HWND(wp.0 as _));
        }
        WM_KEYDOWN if crate::undo::step(|| keys::handle(h, wp.0 as u16)) => {}
        WM_SIZE if !layout::sizing(h) => render(h),
        WM_WINDOWPOSCHANGING if !top() && !crate::visibility::peeking() => keep_bottom(lp),
        WM_GETMINMAXINFO => {
            let i = unsafe { &mut *(lp.0 as *mut MINMAXINFO) };
            let (len, bar) = (scale(h, Fence::MIN.0), layout::bar(h));
            i.ptMinTrackSize = if read(h, |f| f.vertical) == Some(true) { POINT { x: bar, y: len } } else { POINT { x: len, y: bar } };
        }
        WM_ENTERSIZEMOVE => snap::begin(h),
        WM_EXITSIZEMOVE => crate::undo::step(|| {
            snap::end(h);
            layout::persist(h);
            wallpaper::moved(h);
        }),
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
        WM_NCLBUTTONDOWN
            if wp.0 as u32 == HTCAPTION && (filter::bar_click(h, xy(lp)) || browse::bar_click(h, xy(lp)) || tabs::click(h, xy(lp))) => {}
        WM_NCLBUTTONDOWN => return snap::press(h, xy(lp), || unsafe { DefWindowProcW(h, m, wp, lp) }),
        WM_NCLBUTTONDBLCLK if wp.0 as u32 == HTCAPTION && !browse::bar_double(h, xy(lp)) => peek::toggle(h),
        WM_NCMOUSEMOVE => {
            track_nc_leave(h);
            peek::enter(h);
            tabs::hover(h, xy(lp));
        }
        WM_NCRBUTTONUP if wp.0 as u32 == HTCAPTION => peek::keep_open(h, || crate::undo::step(|| menu::context(h))),
        WM_LBUTTONDBLCLK if crate::undo::step(|| columns::down(h, xy(lp))) => {}
        WM_LBUTTONDBLCLK => items::item_at(h, xy(lp)).iter().for_each(|p| browse::open(h, p, wp.0 & MK_CONTROL != 0)),
        WM_LBUTTONDOWN => crate::undo::step(|| {
            if !columns::down(h, xy(lp)) {
                select::down(h, xy(lp), wp.0 & MK_CONTROL != 0);
            }
        }),
        WM_LBUTTONUP | WM_CAPTURECHANGED if !columns::up(h) => select::up(h),
        WM_SETCURSOR if (lp.0 & 0xFFFF) as u32 == HTCLIENT && columns::cursor(h) => return LRESULT(1),
        WM_CONTEXTMENU => peek::keep_open(h, || crate::undo::step(|| menu::context(h))),
        WM_MOUSEMOVE => {
            track_leave(h);
            tabs::off_bar(h);
            peek::enter(h);
            if !columns::track(h, xy(lp)) && !select::track(h, xy(lp)) {
                items::hover(h, items::index_at(h, xy(lp)));
            }
        }
        WM_MOUSELEAVE => {
            items::hover(h, None);
            peek::check(h);
        }
        WM_NCMOUSELEAVE => peek::check(h),
        WM_MOUSEWHEEL if wp.0 & MK_CONTROL != 0 => {
            if let Some(f) = fence_of(h) {
                crate::undo::step(|| items::set_icon(h, icons::step(f.look.icon, wheel(wp) > 0)));
            }
        }
        WM_MOUSEWHEEL => {
            let step = wheel(wp) * metrics(h).0 / 120;
            view(h, |v| v.scroll -= step);
            render(h);
        }
        WM_CLOSE => {}
        WM_SYSCOMMAND if wp.0 & 0xFFF0 == SC_CLOSE as usize => {}
        WM_DESTROY => {
            pacer::leave(h);
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
        pacer::WM_FRAME => anim::tick(h),
        WM_TIMER if wp.0 == PREPARE => paint::prepare(h),
        WM_TIMER if wp.0 == CARET => filter::blink(h),
        WM_COMMAND if (wp.0 >> 16) as u32 == EN_KILLFOCUS => edit::finish(h, true),
        _ => return unsafe { DefWindowProcW(h, m, wp, lp) },
    }
    LRESULT(0)
}
