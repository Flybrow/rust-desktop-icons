use super::{SIDE_TOP, mid, rows};
use crate::{
    i18n::T,
    layered::{ACCENT, WHITE, chrome, popup},
    render::{Canvas, flat, premul},
    win::*,
};
use windows::Win32::{Foundation::*, Graphics::Gdi::*};

fn toggle(c: &mut Canvas, (x1, cy): (i32, i32), on: bool, s: &impl Fn(i32) -> i32, clip: (i32, i32)) {
    let track = (x1 - s(40), cy - s(10), x1, cy + s(10));
    c.rrect(track, s(10) as f32, flat(premul(if on { ACCENT } else { 0x55_5555 }, 255)), clip, false);
    let kx = if on { x1 - s(18) } else { x1 - s(38) };
    c.rrect((kx, cy - s(8), kx + s(16), cy + s(8)), s(8) as f32, flat(premul(WHITE, 255)), clip, false);
}

fn swatch(c: &mut Canvas, (x1, cy): (i32, i32), col: Option<u32>, s: &impl Fn(i32) -> i32, clip: (i32, i32)) {
    let sw = (x1 - s(40), cy - s(11), x1, cy + s(11));
    if let Some(col) = col {
        c.rrect(sw, s(6) as f32, flat(premul(col, 255)), clip, false);
    }
    c.rrect(sw, s(6) as f32, flat(premul(WHITE, 80)), clip, true);
}

pub(super) fn render(h: HWND) {
    let s = |v| scale(h, v);
    let close = popup::close_rect(h);
    popup::paint(h, "settings", |frame, a| {
        let (w, ht) = (frame.w, frame.h);
        let (title, bold, normal, small, icons) = (
            a.font(s(18), FW_SEMIBOLD.0),
            a.font(s(12), FW_SEMIBOLD.0),
            a.font(s(13), FW_NORMAL.0),
            a.font(s(11), FW_NORMAL.0),
            a.icon_font(s(15)),
        );
        let p = a.panel.as_ref()?;
        let rows = rows::build(a, p.page);
        let (top, pad, side) = (s(rows::TOP), s(16), s(rows::SIDE));
        let (x0, x_end) = (side + pad, w - pad - s(14));
        let (x_icon, x_label, mid) = (x0 + s(14), x0 + s(46), mid(h));
        let first_line =
            |r: &rows::Row, y: i32, hh: i32| if r.desc.is_empty() { (y, y + hh) } else { (y + s(9), y + s(rows::SECOND_LINE + 1)) };
        let left = DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS;
        let right = DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS;
        let center = DT_CENTER | DT_VCENTER | DT_SINGLELINE;
        let layout: Vec<(i32, i32)> = rows::layout(&rows).iter().map(|&(y, hh)| (top + s(y) - p.scroll, s(hh))).collect();
        let pages: Vec<(i32, i32)> = (0..rows::PAGES.len() as i32)
            .map(|i| (top + s(SIDE_TOP) + i * s(rows::ITEM), top + s(SIDE_TOP) + (i + 1) * s(rows::ITEM)))
            .collect();
        let body = (top, ht);
        frame.text(title, a.t(T::Settings).trim_end_matches('…'), RECT { left: s(22), top: 0, right: w - s(60), bottom: top }, left);
        frame.text(icons, &crate::layered::glyph::CLOSE.to_string(), close, center);
        for ((name, _), &(y0, y1)) in rows::PAGES.iter().zip(&pages) {
            frame.text(normal, a.t(*name), RECT { left: s(52), top: y0, right: side - s(10), bottom: y1 }, left);
        }
        frame.clip((side, top, w, ht));
        for (r, &(y, hh)) in rows.iter().zip(&layout) {
            let (ly0, ly1) = first_line(r, y, hh);
            let rect = RECT { left: x_label, top: y, right: x_end, bottom: y + hh };
            let line = RECT { top: ly0, bottom: ly1, ..rect };
            match &r.ctl {
                rows::Ctl::Header => frame.text(bold, &r.label.to_uppercase(), RECT { left: x0 + s(6), top: y + s(14), ..rect }, left),
                rows::Ctl::Button => {}
                rows::Ctl::Note => {
                    frame.text(normal, &r.label, RECT { top: y + s(10), bottom: y + hh - s(8), ..rect }, DT_LEFT | DT_WORDBREAK)
                }
                rows::Ctl::Value(v) => {
                    frame.text(normal, &r.label, RECT { right: mid, ..line }, left);
                    frame.text(normal, &format!("{v}   ›"), RECT { left: mid, ..line }, right);
                }
                _ => frame.text(normal, &r.label, RECT { right: x_end - s(56), ..line }, left),
            }
        }
        frame.flush();
        chrome::background(frame, p.close_hot, s);
        let mut c = frame.canvas();
        c.rrect((side, top, side + 1, ht - s(12)), 0., flat(premul(WHITE, 26)), body, false);
        for (i, &(y0, y1)) in pages.iter().enumerate() {
            let r = (s(10), y0 + s(3), side - s(10), y1 - s(3));
            if i == p.page {
                c.rrect(r, s(6) as f32, flat(premul(WHITE, 24)), body, false);
                c.rrect((r.0, y0 + s(12), r.0 + s(3), y1 - s(12)), s(2) as f32, flat(premul(ACCENT, 255)), body, false);
            } else if p.page_hover == Some(i) {
                c.rrect(r, s(6) as f32, flat(premul(WHITE, 14)), body, false);
            }
        }
        for (first, last) in rows::cards(&rows) {
            let (y0, y1) = (layout[first].0, layout[last].0 + layout[last].1);
            c.rrect((x0, y0, w - pad, y1), s(8) as f32, flat(premul(WHITE, 12)), body, false);
            c.rrect((x0, y0, w - pad, y1), s(8) as f32, flat(premul(WHITE, 24)), body, true);
            for &(y, _) in &layout[first + 1..=last] {
                c.rrect((x_label, y, w - pad - s(1), y + 1), 0., flat(premul(WHITE, 18)), body, false);
            }
        }
        for (i, (r, &(y, hh))) in rows.iter().zip(&layout).enumerate() {
            if p.hover == Some(i) {
                c.rrect((x0 + s(3), y + s(3), w - pad - s(3), y + hh - s(3)), s(6) as f32, flat(premul(WHITE, 20)), body, false);
            }
            let at = (x_end, y + hh / 2);
            match &r.ctl {
                rows::Ctl::Toggle(on) => toggle(&mut c, at, *on, &s, body),
                rows::Ctl::Swatch(col) => swatch(&mut c, at, *col, &s, body),
                _ => {}
            }
        }
        frame.text_layers(chrome::TEXT, 0, s(1), (0, ht));
        frame.clear_mask();
        for (r, &(y, hh)) in rows.iter().zip(&layout).filter(|(r, _)| !r.desc.is_empty()) {
            let right = match r.ctl {
                rows::Ctl::Toggle(_) | rows::Ctl::Swatch(_) => x_end - s(56),
                _ => x_end,
            };
            let bottom = y + hh - s(5) - if r.reset.is_some() { s(rows::RESET_LINE) } else { 0 };
            let at = RECT { left: x_label, top: y + s(rows::SECOND_LINE), right, bottom };
            frame.text(small, &r.desc, at, DT_LEFT | DT_WORDBREAK | DT_END_ELLIPSIS);
        }
        frame.flush();
        frame.text_layers(chrome::DIM, 0, s(1), body);
        frame.clear_mask();
        frame.unclip();
        for ((_, glyph), &(y0, y1)) in rows::PAGES.iter().zip(&pages) {
            frame.text(icons, &glyph.to_string(), RECT { left: s(20), top: y0, right: s(44), bottom: y1 }, center);
        }
        frame.clip((side, top, w, ht));
        for (r, &(y, hh)) in rows.iter().zip(&layout) {
            if r.ctl == rows::Ctl::Header {
                continue;
            }
            let glyph_box = if r.ctl == rows::Ctl::Note { (y + s(8), y + s(32)) } else { first_line(r, y, hh) };
            frame.text(
                icons,
                &r.glyph.to_string(),
                RECT { left: x_icon, top: glyph_box.0, right: x_icon + s(22), bottom: glyph_box.1 },
                center,
            );
            if r.ctl == rows::Ctl::Button {
                let (ly0, ly1) = first_line(r, y, hh);
                frame.text(normal, &r.label, RECT { left: x_label, top: ly0, right: x_end, bottom: ly1 }, left);
            }
            if r.reset.is_some() {
                let at = RECT { left: x_label, top: y + hh - s(rows::RESET_LINE + 6), right: x_end, bottom: y + hh - s(6) };
                frame.text(small, &format!("↺  {}", a.t(T::ResetDefault)), at, left);
            }
        }
        frame.flush();
        frame.text_layers(chrome::LINK, 0, s(1), body);
        chrome::outline(frame, s);
        Some(())
    });
}
