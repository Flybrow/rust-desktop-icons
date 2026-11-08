use super::{CLASS, items, layout, target};
use crate::{domain::Fence, win::*};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::{MONITOR_DEFAULTTONULL, MonitorFromRect},
        UI::WindowsAndMessaging::*,
    },
    core::*,
};

pub fn footprints() -> Vec<RECT> {
    items::windows().into_iter().filter_map(layout::footprint).collect()
}

pub(super) fn top() -> bool {
    cfg!(debug_assertions) && std::env::var_os("RDI_TOP").is_some()
}

fn owner(raised: bool) -> Option<HWND> {
    if top() || raised { None } else { progman() }
}

pub fn raise(h: HWND, up: bool) {
    let owner = owner(up).map_or(0, |o| o.0 as isize);
    let after = if up || top() { HWND_TOPMOST } else { HWND_BOTTOM };
    unsafe {
        SetWindowLongPtrW(h, GWLP_HWNDPARENT, owner);
        let _ = SetWindowPos(h, Some(after), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

pub fn create(f: &Fence) -> Option<HWND> {
    unsafe {
        let r = RECT { left: f.x, top: f.y, right: f.x + f.w, bottom: f.y + f.h };
        let (x, y) = if MonitorFromRect(&r, MONITOR_DEFAULTTONULL).is_invalid() { (100, 100) } else { (f.x, f.y) };
        let ex = WS_EX_TOOLWINDOW | WS_EX_LAYERED;
        let h = CreateWindowExW(ex, CLASS, PCWSTR::null(), WS_POPUP, x, y, f.w, f.h, owner(false), None, Some(inst()), None).ok()?;
        if f.rolled {
            layout::resize(h, f);
        }
        if top() {
            raise(h, true);
        }
        let _ = ShowWindow(h, SW_SHOWNOACTIVATE);
        target::register(h);
        Some(h)
    }
}
