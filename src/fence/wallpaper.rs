use super::{fence_of, hwnds, view};
use crate::{
    domain::{Fence, color},
    layered::Dib,
    render::mica::{self, MARGIN, Mica},
    win::window_rect,
};
use std::hash::{Hash, Hasher};
use windows::Win32::{
    Foundation::{HWND, POINT},
    Graphics::Gdi::{CreateCompatibleDC, DeleteDC, GdiFlush, PaintDesktop, SelectObject, SetWindowOrgEx},
    UI::WindowsAndMessaging::{GetSystemMetrics, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN},
};

type Rect = (i32, i32, i32, i32);

fn capture((x, y, w, h): Rect) -> Option<Dib> {
    let dib = Dib::new(w.max(1), h.max(1))?;
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
    Some(dib)
}

fn sample(rect: Rect) -> Option<u32> {
    let mut dib = capture(rect)?;
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

pub(super) fn mica(h: HWND, f: &Fence, (w, ht): (i32, i32)) -> Option<Mica> {
    if !f.look.mica {
        return None;
    }
    let r = window_rect(h);
    let mut k = std::hash::DefaultHasher::new();
    (r.left, r.top, w, ht, f.look.color, f.look.alpha).hash(&mut k);
    let key = k.finish();
    if let Some(m) = view(h, |v| v.mica.as_ref().filter(|(k, _)| *k == key).map(|(_, m)| m.clone())).flatten() {
        return Some(m);
    }
    let mut wall = capture((r.left - MARGIN, r.top - MARGIN, w + 2 * MARGIN, ht + 2 * MARGIN))?;
    let m = mica::build(wall.px(), (w, ht), f.look.color, f.look.alpha);
    view(h, |v| v.mica = Some((key, m.clone())));
    Some(m)
}

pub(super) fn moved(h: HWND) {
    if fence_of(h).is_some_and(|f| f.look.wallpaper || f.look.mica) {
        super::render(h);
    }
}

pub fn changed() {
    for h in hwnds() {
        view(h, |v| {
            v.wall = None;
            v.mica = None;
        });
        if fence_of(h).is_some_and(|f| f.look.wallpaper || f.look.mica) {
            super::render(h);
        }
    }
}
