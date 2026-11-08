use super::{fence_of, hwnds, view};
use crate::{
    domain::{Fence, color},
    layered::Dib,
};
use windows::Win32::{
    Foundation::{HWND, POINT},
    Graphics::Gdi::{CreateCompatibleDC, DeleteDC, GdiFlush, PaintDesktop, SelectObject, SetWindowOrgEx},
    UI::WindowsAndMessaging::{GetSystemMetrics, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN},
};

type Rect = (i32, i32, i32, i32);

fn sample((x, y, w, h): Rect) -> Option<u32> {
    let mut dib = Dib::new(w.max(1), h.max(1))?;
    let (vx, vy) = unsafe { (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN)) };
    unsafe {
        let dc = CreateCompatibleDC(None);
        let old = SelectObject(dc, dib.bmp.into());
        let _ = SetWindowOrgEx(dc, x - vx, y - vy, Some(&mut POINT::default()));
        let _ = PaintDesktop(dc);
        let _ = GdiFlush();
        SelectObject(dc, old);
        let _ = DeleteDC(dc);
    }
    let px: Vec<u32> = dib.px().iter().step_by(7).copied().collect();
    color::average(&px)
}

pub(super) fn color(h: HWND, f: &Fence) -> Option<u32> {
    if !f.look.wallpaper {
        return None;
    }
    let key = (f.x, f.y, f.w, f.h);
    if let Some((_, c)) = view(h, |v| v.wall).flatten().filter(|(k, _)| *k == key) {
        return Some(c);
    }
    let c = color::from_wallpaper(sample(key)?);
    view(h, |v| v.wall = Some((key, c)));
    Some(c)
}

pub(super) fn styled(h: HWND, f: &Fence) -> Option<Fence> {
    let c = color(h, f)?;
    let mut f = f.clone();
    f.look.color = c;
    Some(f)
}

pub fn changed() {
    for h in hwnds() {
        view(h, |v| v.wall = None);
        if fence_of(h).is_some_and(|f| f.look.wallpaper) {
            super::render(h);
        }
    }
}
