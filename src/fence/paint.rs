use super::{PREPARE, anim, cell, fence_of, header, label_font, layout, metrics, title_font, view};
use crate::{
    app::with,
    domain::{self, color, grid},
    layered::{ACCENT, Dib, Frame},
    render::{blit_fit, flat, opaque_if_flat, premul, tint},
    shell,
    win::*,
};
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
    if anim::animating(h) {
        return store(h, None);
    }
    let wr = window_rect(h);
    let size = (wr.right - wr.left, wr.bottom - wr.top);
    let Some(f) = fence_of(h) else { return };
    let Some((frame, alpha)) = draw_fence(h, &f, size, false) else { return };
    frame.present(h, (wr.left, wr.top), alpha);
    let open = view(h, |v| v.unroll == 1.) == Some(true);
    if open && layout::open_size(h, &f) == size {
        store(h, Some(frame));
    } else {
        store(h, None);
        if anim::cached(h) {
            unsafe { SetTimer(Some(h), PREPARE, 150, None) };
        }
    }
}

pub(super) fn prepare(h: HWND) {
    let _ = unsafe { KillTimer(Some(h), PREPARE) };
    if anim::animating(h) || view(h, |v| v.frame.is_some()) != Some(false) {
        return;
    }
    if let Some(f) = fence_of(h).filter(|_| anim::cached(h)) {
        store(h, draw_fence(h, &f, layout::open_size(h, &f), true).map(|(fr, _)| fr));
    }
}

type Shape = ((i32, i32, i32, i32, i32, isize), (i32, i32, i32, i32));

fn labels_key(cells: &[(usize, i32, i32)], v: &crate::app::View, frame: Shape) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut k = std::hash::DefaultHasher::new();
    frame.hash(&mut k);
    for &(i, x, y) in cells {
        (x, y, &v.items[i].name).hash(&mut k);
    }
    k.finish()
}

pub(super) fn draw(h: HWND, size: (i32, i32), open: bool) -> Option<(Frame, u8)> {
    draw_fence(h, &fence_of(h)?, size, open)
}

fn draw_fence(h: HWND, f: &domain::Fence, size: (i32, i32), open: bool) -> Option<(Frame, u8)> {
    match super::wallpaper::styled(h, f) {
        Some(styled) => compose(h, &styled, size, open),
        None => compose(h, f, size, open),
    }
}

fn compose(h: HWND, f: &domain::Fence, (w, ht): (i32, i32), open: bool) -> Option<(Frame, u8)> {
    let s = |v| scale(h, v);
    let (cell, icon) = metrics(h);
    let (mut frame, mut icons) = (Frame::new(w, ht)?, Dib::new(w, ht)?);
    let g = layout::geo(h, (w, ht));
    let opened = layout::geo(h, layout::open_size(h, f));
    let (bx0, by0, bx1, by1) = g.body();
    let gw = g.grid_w();
    let shape = super::shape(h, gw);
    let mica = super::wallpaper::mica(h, f, (w, ht));
    let alpha = with(|a| {
        let (ft, fi, glyphs, empty) = (title_font(a, h), label_font(a, h), a.icon_font(s(13)), a.t(crate::i18n::T::FilterHint));
        let peeking = crate::visibility::peeking();
        let mask =
            a.view(h).map(|v| v.inside).map(|inside| a.cfg.privacy.mask(inside, crate::visibility::privacy::desktop_active(), peeking));
        let mask = mask.unwrap_or_default();
        let flow = layout::flow(a, h, gw, cell)?;
        let tail = layout::tail(a, h, gw, (cell, icon));
        let v = a.view(h)?;
        let rolled = !open && v.unroll == 0.;
        let view_h = if !open && v.unroll < 1. { opened.view_h() } else { g.view_h() };
        let max = if rolled { 0 } else { grid::max_scroll(v.items.len(), shape.span(gw), view_h, cell, tail) };
        v.scroll = v.scroll.clamp(0, max);
        let cells: Vec<(usize, i32, i32)> = (0..if rolled { 0 } else { v.items.len() })
            .map(|i| (i, flow.origin(i)))
            .map(|(i, (x, y))| (i, g.cell((x, y - v.scroll), (shape.w, shape.h))))
            .map(|(i, (x, y))| (i, x, y))
            .filter(|&(_, x, y)| y + shape.h > by0 && y < by1 && x + shape.w > bx0 && x < bx1)
            .collect();
        frame.clip((bx0, by0, bx1, by1));
        let key = (f.look.labels && !mask.names)
            .then(|| labels_key(&cells, v, ((w, ht, cell, icon, s(100), fi.0 as isize), (bx0, by0, bx1, by1))));
        let cached = key.and_then(|k| v.labels.take().filter(|l| l.key == k));
        for &(i, x, y) in cells.iter().filter(|_| key.is_some() && cached.is_none()) {
            frame.text_w(fi, &mut v.items[i].name, &mut cell::label_at((x, y), shape, icon, s), shape.flags());
        }
        frame.target(&icons);
        let thumb = |i: usize| v.items[i].thumb.as_ref().filter(|_| f.look.thumbs);
        for &(i, x, y) in cells.iter().filter(|c| !mask.icons && thumb(c.0).is_none()) {
            shell::draw_icon(frame.dc(), v.items[i].icon, cell::icon_at((x, y), shape, icon, s), icon);
        }
        frame.target(&frame.mask);
        if cached.is_some() || key.is_none() {
            frame.sync()
        } else {
            frame.flush()
        }
        for &(i, x, y) in cells.iter().filter(|_| !mask.icons) {
            let (ix, iy) = cell::icon_at((x, y), shape, icon, s);
            match thumb(i) {
                Some(t) => blit_fit(icons.px(), w, &t.px, (t.w, t.h), (ix, iy), icon),
                None => opaque_if_flat(icons.px(), w, (ix, iy, ix + icon, iy + icon)),
            }
        }
        if let Some(c) = f.look.tint {
            tint(icons.px(), c);
        }
        let under = mica.as_ref().map_or(f.look.color, |m| m.average());
        let (body, (fg, shade), ink) = (f.look.alpha, color::text_on(under), color::ink(under));
        let bar = !(f.look.bar_hover && !f.rolled && !v.inside);
        let (full, rad, rows) = (if bar { (0, 0, w, ht) } else { (bx0, by0, bx1, by1) }, s(f.look.corner.px()) as f32, (0, ht));
        let tone = |k, a: u8| premul(color::shade(f.look.color, k), a);
        let mut c = frame.canvas();
        match &mica {
            Some(m) => c.rrect_with(full, rad, rows, false, |x, y| m.at(x, y)),
            None => c.rrect(full, rad, (tone(10, body), tone(-14, body.saturating_add(20))), rows, false),
        }
        for &(i, x, y) in &cells {
            let (sel, hot) = (v.selected.contains(&v.items[i].path), v.hover == Some(i));
            if sel || hot {
                let label = if f.look.labels {
                    crate::layered::measure(fi, &v.items[i].name, cell::label_width(shape, s), shape.flags())
                } else {
                    0
                };
                let r = cell::tile((x, y), shape, icon, label, s);
                let (fill, line) = if sel { (premul(ACCENT, 90), premul(ACCENT, 200)) } else { (premul(ink, 40), premul(ink, 60)) };
                c.rrect(r, rad, flat(fill), (by0, by1), false);
                c.rrect(r, rad, flat(line), (by0, by1), true);
            }
        }
        for &(_, x, y) in cells.iter().filter(|_| mask.any()) {
            if mask.icons {
                let (ix, iy) = cell::icon_at((x, y), shape, icon, s);
                c.rrect((ix, iy, ix + icon, iy + icon), s(8) as f32, flat(premul(ink, 46)), (by0, by1), false);
            }
            if mask.names && f.look.labels {
                let r = cell::label_at((x, y), shape, icon, s);
                let (pad, top) = ((r.right - r.left) / 5, r.top + s(4));
                c.rrect((r.left + pad, top, r.right - pad, top + s(8)), s(4) as f32, flat(premul(ink, 70)), (by0, by1), false);
            }
        }
        if let Some(((x0, y0), (x1, y1))) = v.band {
            let r = g.rect((x0.min(x1), y0.min(y1) - v.scroll, x0.max(x1), y0.max(y1) - v.scroll));
            c.rrect(r, s(2) as f32, flat(premul(ACCENT, 50)), (by0, by1), false);
            c.rrect(r, s(2) as f32, flat(premul(ACCENT, 200)), (by0, by1), true);
        }
        c.layer(icons.px(), (0, 0), None, (by0, by1));
        if max > 0 {
            let avail = g.view_h();
            let th = (avail * avail / (avail + max)).max(s(24));
            let ty = v.scroll * (avail - th) / max;
            let r = g.rect((gw - s(7), ty + s(3), gw - s(3), ty + th - s(3)));
            c.rrect(r, s(2) as f32, flat(premul(ink, 110)), (by0, by1), false);
        }
        let labels = cached.or_else(|| key.map(|k| frame.labels(k, s(2))));
        if let Some(l) = &labels {
            frame.layers(l, f.look.label_color.unwrap_or(fg), shade, rows);
        }
        v.labels = labels.filter(|_| v.inside);
        let hint = super::filter::hint(v, empty);
        let hint = hint.or_else(|| {
            v.hover.filter(|_| !f.look.labels && !mask.names).and_then(|i| v.items.get(i)).map(|it| String::from_utf16_lossy(&it.name))
        });
        let hint = hint.or_else(|| v.browse.last().map(|p| format!("←  {}", crate::store::name(p))));
        if bar {
            let loupe = ((v.inside && !rolled && f.active().portal.is_some()) || v.filter.is_some()).then_some(glyphs);
            let hidden = mask.titles.then(|| {
                let mut m = f.clone();
                m.tabs.iter_mut().for_each(|t| t.title = "•••".into());
                m
            });
            let (hf, hint) = (hidden.as_ref().unwrap_or(f), hint.filter(|_| !mask.titles));
            header::paint(&mut frame, &g, (ft, loupe), (hf, hint.as_deref()), (!rolled).then_some(f.tab), s);
        }
        frame.canvas().rrect(full, rad, flat(premul(ink, 48)), rows, true);
        #[cfg(debug_assertions)]
        frame.dump(&f.id.to_string());
        Some(domain::anim::opacity(v.glow))
    });
    alpha.flatten().map(|a| (frame, a))
}
