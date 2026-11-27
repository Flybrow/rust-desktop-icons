use super::super::{bar_font, cell, columns, detail, header, label_font, layout, metrics, title_font, wallpaper::Glass};
use super::{Sizes, marks};
use crate::{
    app::with,
    domain::{self, color, columns::Mode, grid},
    layered::{Dib, Frame},
    render::{blit_fit, flat, opaque_if_flat, premul, tint},
    shell,
    win::*,
};
use windows::Win32::{Foundation::*, Graphics::Gdi::FW_NORMAL};

type Shape = ((i32, i32, i32, i32, i32, isize), (i32, i32, i32, i32));

fn labels_key(cells: &[(usize, i32, i32)], v: &crate::app::View, frame: Shape, extra: impl std::hash::Hash) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut k = std::hash::DefaultHasher::new();
    (frame, extra).hash(&mut k);
    for &(i, x, y) in cells {
        (x, y, &v.items[i].name, &v.items[i].info).hash(&mut k);
    }
    k.finish()
}

pub(super) fn compose(h: HWND, f: &domain::Fence, ((w, ht), opened): Sizes, open: bool, frost: Option<&Glass>) -> Option<(Frame, u8)> {
    let s = |v| scale(h, v);
    let (cell, icon) = metrics(h);
    let (mut frame, mut icons) = (Frame::new(w, ht)?, Dib::new(w, ht)?);
    let g = layout::geo(h, (w, ht));
    let opened = layout::geo(h, opened);
    let (bx0, by0, bx1, by1) = g.body();
    let gw = g.grid_w();
    let shape = super::super::shape(h, gw);
    let spans = if shape.mode == Mode::Details { super::super::modes::spans(h, gw) } else { vec![] };
    let (rows, head, sort) = (g.rows(), g.head_rect(), (f.active().sort, f.active().desc));
    let (rx0, ry0, rx1, ry1) = rows;
    let alpha = with(|a| {
        let pre = detail::Prefix { modified: a.t(crate::i18n::T::SortDate).into(), size: a.t(crate::i18n::T::SortSize).into() };
        let titles = spans.iter().map(|sp| a.t(crate::i18n::column(sp.0)).to_string()).collect();
        let header = columns::Header { spans: &spans, sort, titles };
        let (ft, fi, glyphs, empty) = (title_font(a, h), label_font(a, h), a.icon_font(s(13)), a.t(crate::i18n::T::FilterHint));
        let fq = bar_font(a, h, FW_NORMAL.0);
        let peeking = crate::visibility::peeking();
        let mask =
            a.view(h).map(|v| v.inside).map(|inside| a.cfg.privacy.mask(inside, crate::visibility::privacy::desktop_active(), peeking));
        let mask = mask.unwrap_or_default();
        let flow = layout::flow(a, h, gw, shape)?;
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
            .filter(|&(_, x, y)| y + shape.h > ry0 && y < ry1 && x + shape.w > rx0 && x < rx1)
            .collect();
        frame.clip(rows);
        let named = f.look.labels || !shape.icons();
        let extra = (mask.names, shape.mode, &spans, sort);
        let key = (named && (!mask.names || g.head > 0))
            .then(|| labels_key(&cells, v, ((w, ht, cell, icon, s(100), fi.0 as isize), (bx0, by0, bx1, by1)), extra));
        let cached = key.and_then(|k| v.labels.take().filter(|l| l.key == k));
        let mut dims = vec![];
        for &(i, x, y) in cells.iter().filter(|_| key.is_some() && cached.is_none() && !mask.names) {
            frame.text_w(fi, &mut v.items[i].name, &mut cell::label_at((x, y), shape, icon, s), shape.flags());
            for (mut t, mut r, flags) in detail::lines(&v.items[i], (x, y), shape, icon, (&spans, &pre), s) {
                frame.text_w(fi, &mut t, &mut r, flags);
                dims.push((r.left.max(rx0), r.top.max(ry0), r.right.min(rx1), r.bottom.min(ry1)));
            }
        }
        if g.head > 0 && key.is_some() && cached.is_none() {
            frame.unclip();
            columns::text(&frame, (fi, glyphs), head, &header, s);
            frame.clip(rows);
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
        dims.iter().for_each(|r| frame.dim(*r, 150));
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
        let under = f.look.color;
        let (body, (fg, shade), ink) = (f.look.alpha, color::text_on(under), color::ink(under));
        let bar = !(f.look.bar_hover && !f.rolled && !v.inside);
        let (full, rad, all) = (if bar { (0, 0, w, ht) } else { (bx0, by0, bx1, by1) }, s(f.look.corner.px()) as f32, (0, ht));
        let tone = |k, a: u8| premul(color::shade(f.look.color, k), a);
        let mut c = frame.canvas();
        if f.look.glass {
            match frost {
                Some((fr, (ox, oy))) => c.rrect_with(full, rad, all, false, |x, y| fr.at(x + ox, y + oy)),
                None => c.rrect(full, rad, (tone(16, body), tone(0, (body as u32 * 3 / 5) as u8)), all, false),
            }
            if ink != 0 {
                c.rrect(full, rad, flat(premul(0xFF_FFFF, 14)), all, false);
            }
        } else {
            c.rrect(full, rad, (tone(10, body), tone(-14, body.saturating_add(20))), all, false);
        }
        if g.head > 0 && !rolled {
            columns::shapes(&mut c, head, &spans, ink, s);
        }
        let glass = f.look.glass.then_some(ink == 0);
        let k = marks::Ctx { shape, icon, ink, rad, glass, rows: (ry0, ry1), s: &s };
        marks::highlights(&mut c, &cells, v, (fi, f.look.labels), &k);
        marks::masks(&mut c, &cells, mask, named, &k);
        marks::band(&mut c, &g, v, &k);
        c.layer(icons.px(), (0, 0), None, (ry0, ry1));
        marks::scrollbar(&mut c, &g, (v.scroll, max, gw), &k);
        let labels = cached.or_else(|| key.map(|k| frame.labels(k, s(2))));
        if let Some(l) = &labels {
            match f.look.glass {
                true => {
                    let (gfg, gshade) = color::glass_text(under);
                    frame.layers_with(l, f.look.label_color.unwrap_or(gfg), gshade, all)
                }
                false => frame.layers(l, f.look.label_color.unwrap_or(fg), shade, all),
            }
        }
        v.labels = labels.filter(|_| v.inside);
        let field = super::super::filter::field(h, v, empty);
        let hint = v.hover.filter(|_| !named && !mask.names).and_then(|i| v.items.get(i)).map(|it| String::from_utf16_lossy(&it.name));
        let hint = hint.or_else(|| v.browse.last().map(|p| format!("←  {}", crate::store::name(p))));
        if bar {
            let loupe = (v.inside && !rolled && f.active().portal.is_some()).then_some(!v.browse.is_empty());
            let hidden = mask.titles.then(|| {
                let mut m = f.clone();
                m.tabs.iter_mut().for_each(|t| t.title = "•••".into());
                m
            });
            let (hf, hint) = (hidden.as_ref().unwrap_or(f), hint.filter(|_| !mask.titles));
            let tabs = (!rolled).then_some(f.tab);
            header::paint(&mut frame, &g, (ft, fq, glyphs, loupe), (hf, hint.as_deref()), tabs, field.as_ref(), s);
        }
        let mut c = frame.canvas();
        if f.look.glass {
            let light = ink == 0;
            if light {
                c.rrect(full, rad, flat(premul(0, 14)), all, true);
            }
            let top = (full.0 + rad as i32, full.1 + 1, full.2 - rad as i32, full.1 + 2);
            c.rrect(top, 0., flat(premul(0xFF_FFFF, if light { 150 } else { 18 })), all, false);
        } else {
            c.rrect(full, rad, flat(premul(ink, 48)), all, true);
        }
        #[cfg(debug_assertions)]
        if !open {
            frame.dump(&f.id.to_string());
        }
        Some(domain::anim::opacity(v.glow))
    });
    alpha.flatten().map(|a| (frame, a))
}
