use super::{fence_of, hwnds, layout, view};
use crate::{
    domain::{Fence, color},
    layered::Dib,
    render::frost::{self, Frost, MARGIN},
};
use std::rc::Rc;
use windows::Win32::{
    Foundation::{HWND, POINT},
    Graphics::Gdi::{CreateCompatibleDC, DeleteDC, GdiFlush, PaintDesktop, SelectObject, SetWindowOrgEx},
};

type Rect = (i32, i32, i32, i32);
pub(super) type Glass = (Rc<Frost>, (i32, i32));

fn capture((x, y, w, h): Rect) -> Option<Dib> {
    let dib = Dib::new(w.max(1), h.max(1))?;
    unsafe {
        let dc = CreateCompatibleDC(None);
        let old = SelectObject(dc, dib.bmp.into());
        let _ = SetWindowOrgEx(dc, x, y, Some(&mut POINT::default()));
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

fn spot(h: HWND, f: &Fence) -> Rect {
    layout::area(h, f, (f.w, f.h))
}

fn color(h: HWND, f: &Fence) -> Option<u32> {
    if !f.look.wallpaper {
        return None;
    }
    let key = spot(h, f);
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

pub(super) fn frost(h: HWND, f: &Fence, at: (i32, i32), opened: (i32, i32)) -> Option<Glass> {
    if !f.look.glass {
        return None;
    }
    let (rect, look) = (layout::area(h, f, opened), (f.look.color, f.look.alpha));
    let hit = view(h, |v| {
        let sizing = v.drag.is_some();
        v.frost.as_ref().filter(|(l, r, _)| *l == look && (*r == rect || sizing)).map(|(_, r, m)| (m.clone(), *r))
    });
    let (m, (x, y, ..)) = match hit.flatten() {
        Some(c) => c,
        None => {
            let (x, y, w, ht) = rect;
            let mut wall = capture((x - MARGIN, y - MARGIN, w + 2 * MARGIN, ht + 2 * MARGIN))?;
            let strength = frost::strength(f.look.alpha, color::ink(f.look.color) == 0);
            let m = Rc::new(frost::build(wall.px(), (w, ht), f.look.color, strength));
            view(h, |v| v.frost = Some((look, rect, m.clone())));
            (m, rect)
        }
    };
    Some((m, (at.0 - x, at.1 - y)))
}

fn behind(f: &Fence) -> bool {
    f.look.wallpaper || f.look.glass
}

pub(super) fn moved(h: HWND) {
    let Some(f) = fence_of(h).filter(behind) else { return };
    let (rect, at) = (layout::area(h, &f, layout::open_size(h, &f)), spot(h, &f));
    let stale = view(h, |v| {
        let glass = f.look.glass && v.frost.as_ref().is_none_or(|c| c.1 != rect);
        glass || (f.look.wallpaper && v.wall.is_none_or(|w| w.0 != at))
    });
    if stale == Some(true) {
        super::render(h);
    }
}

pub fn changed() {
    for h in hwnds() {
        view(h, |v| (v.wall, v.frost) = (None, None));
        if fence_of(h).is_some_and(|f| behind(&f)) {
            super::render(h);
        }
    }
}
