use super::{fence_of, layout, pacer, paint, read, view};
use crate::app::View;
use crate::{
    app::with,
    domain::{Fence, anim},
    layered::Frame,
    win::now_ms,
};
use windows::Win32::Foundation::HWND;

fn targets((rolled, chameleon): (bool, bool), v: &View) -> (f32, f32) {
    let unroll = if !rolled || (v.inside && !v.hold) { 1. } else { 0. };
    (unroll, if !chameleon || v.inside { 1. } else { 0. })
}

pub(super) fn start(h: HWND) {
    crate::perf::mark("start", h.0 as isize, 0.);
    let side = layout::side(h);
    layout::orient(h);
    let still = with(|a| {
        let kind = a.fence_of(h).map(|f| (f.rolled, f.look.chameleon))?;
        let v = a.view(h)?;
        Some(v.tick.is_none() && v.grow.is_none() && targets(kind, v) == (v.unroll, v.glow))
    })
    .flatten();
    if still == Some(true) && layout::side(h) == side {
        return;
    }
    let idle = view(h, |v| v.tick.replace(v.tick.unwrap_or_else(now_ms)).is_none());
    if idle == Some(true) {
        pacer::join(h);
    }
}

pub(super) fn extent(h: HWND, f: &Fence) -> Option<i32> {
    let (unroll, tick, grow) = view(h, |v| (v.unroll, v.tick, v.grow))?;
    if unroll != 1. || with(|a| a.cfg.roll_ms) == Some(0) || super::browse::current(h).is_some() {
        return None;
    }
    let full = layout::full(h, f);
    match grow {
        Some((from, q)) => Some(anim::between(from, full, q)),
        None => tick.is_none().then_some(full),
    }
}

pub(super) fn grow(h: HWND, from: i32) {
    let Some(f) = fence_of(h) else { return };
    if layout::full(h, &f) == from && view(h, |v| v.grow.is_none()) == Some(true) {
        return layout::apply(h);
    }
    view(h, |v| (v.grow, v.stale) = (Some((from, 0.)), true));
    let sizes = layout::sizes(h, &f);
    if let Some((frame, alpha)) = paint::draw(h, sizes.0) {
        show(h, &f, &frame, sizes, alpha);
        view(h, |v| v.frame = Some(frame));
    }
    start(h);
}

fn show(h: HWND, f: &Fence, frame: &Frame, (open, size): ((i32, i32), (i32, i32)), alpha: u8) {
    let (pos, src) = (layout::origin(h, f, size), layout::crop(h, open, size));
    layout::self_sized(h, || frame.present_part(h, pos, src, size, alpha));
}

pub(super) fn cached(h: HWND) -> bool {
    read(h, |f| f.rolled || f.look.chameleon) == Some(true)
}

pub(super) fn animating(h: HWND) -> bool {
    view(h, |v| v.tick.is_some()) == Some(true)
}

pub(super) fn tick(h: HWND) {
    pacer::ticked(h);
    let t = now_ms();
    let Some((f, done, alpha)) = with(|a| {
        let ms = a.cfg.roll_ms;
        let f = a.fence_of(h)?.clone();
        let v = a.view(h)?;
        let dt = anim::step((t - v.tick?) as f32);
        if dt < 1. {
            return None;
        }
        let (unroll, glow) = targets((f.rolled, f.look.chameleon), v);
        if unroll != v.unroll {
            v.grow = None;
        }
        let opening = unroll > v.unroll;
        if opening != v.opening && v.unroll > 0. && v.unroll < 1. {
            v.unroll = anim::retarget(v.unroll, v.opening, opening);
        }
        v.opening = opening;
        v.unroll = anim::advance(v.unroll, unroll, dt, ms);
        v.glow = anim::advance(v.glow, glow, dt, ms);
        if let Some(g) = v.grow.as_mut() {
            g.1 = anim::advance(g.1, 1., dt, ms);
        }
        v.grow = v.grow.filter(|g| g.1 < 1.);
        let done = v.unroll == unroll && v.glow == glow && v.grow.is_none();
        v.tick = (!done).then_some(t);
        Some((f, done, anim::opacity(v.glow)))
    })
    .flatten() else {
        return;
    };
    let (open, size) = layout::sizes(h, &f);
    if done {
        pacer::leave(h);
        if f.rolled && view(h, |v| v.unroll == 0.) == Some(true) {
            super::browse::home(h);
        }
    }
    let cached = with(|a| a.view(h)?.frame.take()).flatten().filter(|fr| (fr.w, fr.h) == open);
    crate::perf::mark(if cached.is_some() { "tick.cached" } else { "tick.miss" }, h.0 as isize, 0.);
    if done && size != open {
        let stale = view(h, |v| std::mem::take(&mut v.stale)) == Some(true);
        crate::perf::span("tick.apply", h.0 as isize, || layout::apply(h));
        return paint::store(h, cached.filter(|_| !stale));
    }
    let Some(frame) = cached.or_else(|| crate::perf::span("tick.draw", h.0 as isize, || paint::draw(h, open).map(|(fr, _)| fr))) else {
        return;
    };
    crate::perf::span("tick.present", h.0 as isize, || show(h, &f, &frame, (open, size), alpha));
    match done {
        true => paint::store(h, Some(frame)),
        false => view(h, |v| v.frame = Some(frame)).unwrap_or_default(),
    }
    if done && view(h, |v| std::mem::take(&mut v.stale)) == Some(true) {
        paint::render(h);
    }
}
