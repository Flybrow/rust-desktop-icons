mod compose;
mod marks;

use super::{PREPARE, anim, fence_of, layout, view};
use crate::{domain, layered::Frame, win::*};
use windows::Win32::{
    Foundation::*,
    UI::WindowsAndMessaging::{KillTimer, SetTimer},
};

pub(super) fn store(h: HWND, frame: Option<Frame>) {
    let frame = frame.filter(|_| anim::cached(h)).map(|mut fr| {
        fr.compact();
        fr
    });
    view(h, |v| v.frame = frame);
}

pub fn render(h: HWND) {
    crate::perf::span("render", h.0 as isize, || render_now(h))
}

fn render_now(h: HWND) {
    let wr = window_rect(h);
    render_at(h, (wr.left, wr.top), (wr.right - wr.left, wr.bottom - wr.top));
}

fn opened(h: HWND, f: &domain::Fence, open: (i32, i32)) -> Option<(Frame, u8)> {
    draw_fence(h, f, (open, open), (true, layout::origin(h, f, open)))
}

pub(super) fn render_at(h: HWND, pos: (i32, i32), size: (i32, i32)) -> bool {
    if anim::animating(h) {
        crate::perf::mark("render.drop", h.0 as isize, 0.);
        view(h, |v| v.stale = true);
        return true;
    }
    let Some(f) = fence_of(h) else { return false };
    let opened = layout::open_size(h, &f);
    let Some((frame, alpha)) = draw_fence(h, &f, (size, opened), (false, pos)) else { return false };
    frame.present(h, pos, alpha);
    let open = view(h, |v| v.unroll == 1.) == Some(true);
    if open && opened == size {
        store(h, Some(frame));
    } else {
        store(h, None);
        if anim::cached(h) {
            unsafe { SetTimer(Some(h), PREPARE, 150, None) };
        }
    }
    true
}

pub(super) fn prepare(h: HWND) {
    let _ = unsafe { KillTimer(Some(h), PREPARE) };
    if anim::animating(h) || view(h, |v| v.frame.is_some()) != Some(false) {
        return;
    }
    if let Some(f) = fence_of(h).filter(|_| anim::cached(h)) {
        store(h, crate::perf::span("prepare", h.0 as isize, || opened(h, &f, layout::open_size(h, &f))).map(|(fr, _)| fr));
    }
}

pub(super) fn draw(h: HWND, open: (i32, i32)) -> Option<(Frame, u8)> {
    opened(h, &fence_of(h)?, open)
}

type Sizes = ((i32, i32), (i32, i32));

fn draw_fence(h: HWND, f: &domain::Fence, sizes: Sizes, (open, at): (bool, (i32, i32))) -> Option<(Frame, u8)> {
    let styled = super::wallpaper::styled(h, f);
    let f = styled.as_ref().unwrap_or(f);
    let frost = super::wallpaper::frost(h, f, at, sizes.1);
    compose::compose(h, f, sizes, open, frost.as_ref())
}
