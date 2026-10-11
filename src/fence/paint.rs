use super::{PREPARE, anim, cell, fence_of, header, label_font, layout, metrics, title_font, view};
use crate::{
    app::with,
    domain::{self, color, grid},
    layered::{ACCENT, Dib, Frame, WHITE},
    render::{flat, opaque_if_flat, premul, tint},
    shell,
    win::*,
};
use windows::Win32::{
    Foundation::*,
    UI::WindowsAndMessaging::{KillTimer, SetTimer},
};

pub(super) fn store(h: HWND, frame: Option<Frame>) {
    let frame = frame.filter(|_| fence_of(h).is_some_and(|f| anim::cached(h, &f))).map(|mut fr| {
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
    let Some((frame, alpha)) = draw(h, size, false) else { return };
    frame.present(h, (wr.left, wr.top), alpha);
    let open = view(h, |v| v.unroll == 1.) == Some(true);
    if open && fence_of(h).is_some_and(|f| layout::open_size(h, &f) == size) {
        store(h, Some(frame));
    } else {
        store(h, None);
        if fence_of(h).is_some_and(|f| anim::cached(h, &f)) {
            unsafe { SetTimer(Some(h), PREPARE, 150, None) };
        }
    }
}

pub(super) fn prepare(h: HWND) {
    let _ = unsafe { KillTimer(Some(h), PREPARE) };
    if anim::animating(h) || view(h, |v| v.frame.is_some()) != Some(false) {
        return;
    }
    if let Some(f) = fence_of(h).filter(|f| anim::cached(h, f)) {
        store(h, draw(h, layout::open_size(h, &f), true).map(|(fr, _)| fr));
    }
}

pub(super) fn draw(h: HWND, (w, ht): (i32, i32), open: bool) -> Option<(Frame, u8)> {
    let s = |v| scale(h, v);
    let (cell, icon) = metrics(h);
    let (mut frame, mut icons) = (Frame::new(w, ht)?, Dib::new(w, ht)?);
    let g = layout::geo(h, (w, ht));
    let opened = fence_of(h).map_or(g, |f| layout::geo(h, layout::open_size(h, &f)));
    let (bx0, by0, bx1, by1) = g.body();
    let gw = g.grid_w();
    let alpha = with(|a| {
        let (ft, fi) = (title_font(a, h), label_font(a, h));
        let f = a.fence_of(h)?.clone();
        let v = a.view(h)?;
        let rolled = !open && v.unroll == 0.;
        let view_h = if !open && v.unroll < 1. { opened.view_h() } else { g.view_h() };
        let max = if rolled { 0 } else { grid::max_scroll(v.items.len(), gw, view_h, cell) };
        v.scroll = v.scroll.clamp(0, max);
        let cells: Vec<(usize, i32, i32)> = (0..if rolled { 0 } else { v.items.len() })
            .map(|i| (i, grid::origin(i, gw, cell)))
            .map(|(i, (x, y))| (i, g.cell((x, y - v.scroll), cell)))
            .map(|(i, (x, y))| (i, x, y))
            .filter(|&(_, x, y)| y + cell > by0 && y < by1 && x + cell > bx0 && x < bx1)
            .collect();
        frame.clip((bx0, by0, bx1, by1));
        for &(i, x, y) in &cells {
            frame.text_w(fi, &mut v.items[i].name, &mut cell::label_at((x, y), cell, icon, s), cell::LABEL);
        }
        frame.target(&icons);
        for &(i, x, y) in &cells {
            shell::draw_icon(frame.dc(), v.items[i].icon, cell::icon_at((x, y), cell, icon, s), icon);
        }
        frame.flush();
        for &(_, x, y) in &cells {
            let (ix, iy) = cell::icon_at((x, y), cell, icon, s);
            opaque_if_flat(icons.px(), w, (ix, iy, ix + icon, iy + icon));
        }
        if let Some(c) = f.look.tint {
            tint(icons.px(), c);
        }
        let (body, fg) = (f.look.alpha, color::contrast(f.look.color));
        let shade = if fg == WHITE || fg > 0x80_8080 { 0 } else { WHITE };
        let (full, rad, rows) = ((0, 0, w, ht), s(8) as f32, (0, ht));
        let tone = |k, a: u8| premul(color::shade(f.look.color, k), a);
        let mut c = frame.canvas();
        c.rrect(full, rad, (tone(10, body), tone(-14, body.saturating_add(20))), rows, false);
        for &(i, x, y) in &cells {
            let (sel, hot) = (v.selected.contains(&v.items[i].path), v.hover == Some(i));
            if sel || hot {
                let r = (x + s(5), y + s(3), x + cell - s(5), y + cell - s(1));
                let (fill, line) = if sel { (premul(ACCENT, 90), premul(ACCENT, 200)) } else { (premul(WHITE, 40), premul(WHITE, 60)) };
                c.rrect(r, rad, flat(fill), (by0, by1), false);
                c.rrect(r, rad, flat(line), (by0, by1), true);
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
            c.rrect(r, s(2) as f32, flat(premul(WHITE, 110)), (by0, by1), false);
        }
        frame.text_layers(fg, shade, s(2), rows);
        header::paint(&mut frame, &g, ft, &f, (!rolled).then_some(f.tab), (fg, shade), s);
        frame.canvas().rrect(full, rad, flat(premul(WHITE, 48)), rows, true);
        #[cfg(debug_assertions)]
        frame.dump(&f.id.to_string());
        Some(domain::anim::opacity(v.glow))
    });
    alpha.flatten().map(|a| (frame, a))
}
