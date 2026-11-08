use super::{ANIM, layout, paint, read, view};
use crate::app::View;
use crate::{app::with, domain::anim, win::now_ms};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{KillTimer, SetTimer},
};

fn targets((rolled, chameleon): (bool, bool), v: &View) -> (f32, f32) {
    let unroll = if !rolled || (v.inside && !v.hold) { 1. } else { 0. };
    (unroll, if !chameleon || v.inside { 1. } else { 0. })
}

pub(super) fn start(h: HWND) {
    let side = layout::side(h);
    layout::orient(h);
    let still = with(|a| {
        let kind = a.fence_of(h).map(|f| (f.rolled, f.look.chameleon))?;
        let v = a.view(h)?;
        Some(v.tick.is_none() && targets(kind, v) == (v.unroll, v.glow))
    })
    .flatten();
    if still == Some(true) && layout::side(h) == side {
        return;
    }
    let idle = view(h, |v| v.tick.replace(v.tick.unwrap_or_else(now_ms)).is_none());
    if idle == Some(true) {
        unsafe { SetTimer(Some(h), ANIM, 10, None) };
    }
}

pub(super) fn cached(h: HWND) -> bool {
    let Some((rolled, chameleon)) = read(h, |f| (f.rolled, f.look.chameleon)) else { return false };
    let busy = view(h, |v| v.inside || v.tick.is_some()).unwrap_or(false);
    if rolled { busy } else { chameleon }
}

pub(super) fn animating(h: HWND) -> bool {
    view(h, |v| v.tick.is_some()) == Some(true)
}

pub(super) fn tick(h: HWND) {
    let t = now_ms();
    let Some((f, done, alpha)) = with(|a| {
        let ms = a.cfg.roll_ms;
        let f = a.fence_of(h)?.clone();
        let v = a.view(h)?;
        let dt = (t - v.tick?) as f32;
        if dt < 1. {
            return None;
        }
        let (unroll, glow) = targets((f.rolled, f.look.chameleon), v);
        let opening = unroll > v.unroll;
        if opening != v.opening && v.unroll > 0. && v.unroll < 1. {
            v.unroll = anim::retarget(v.unroll, v.opening, opening);
        }
        v.opening = opening;
        v.unroll = anim::advance(v.unroll, unroll, dt, ms);
        v.glow = anim::advance(v.glow, glow, dt, ms);
        let done = v.unroll == unroll && v.glow == glow;
        v.tick = (!done).then_some(t);
        Some((f, done, anim::opacity(v.glow)))
    })
    .flatten() else {
        return;
    };
    let (open, size) = (layout::open_size(h, &f), layout::size(h, &f));
    if done {
        let _ = unsafe { KillTimer(Some(h), ANIM) };
        if f.rolled && view(h, |v| v.unroll == 0.) == Some(true) {
            super::browse::home(h);
        }
    }
    let cached = with(|a| a.view(h)?.frame.take()).flatten().filter(|fr| (fr.w, fr.h) == open);
    if done && size != open {
        return layout::apply(h);
    }
    let Some(frame) = cached.or_else(|| paint::draw(h, open, true).map(|(fr, _)| fr)) else {
        return;
    };
    let (pos, src) = (layout::origin(h, &f, size), layout::crop(h, open, size));
    layout::self_sized(h, || frame.present_part(h, pos, src, size, alpha));
    paint::store(h, Some(frame));
}
