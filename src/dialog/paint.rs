use super::{
    APP, HEAD,
    layout::{Layout, fill},
    notes::{Ink, Line},
};
use crate::{
    layered::{ACCENT, Frame, chrome, glyph as g, line_width, logo, popup},
    render::{blend, flat, lerp, premul},
    win::*,
};
use windows::Win32::{Foundation::*, Graphics::Gdi::*};

const WRAP: DRAW_TEXT_FORMAT = DRAW_TEXT_FORMAT(DT_LEFT.0 | DT_WORDBREAK.0 | DT_NOPREFIX.0 | DT_EXPANDTABS.0);
const SINGLE: DRAW_TEXT_FORMAT = DRAW_TEXT_FORMAT(DT_VCENTER.0 | DT_SINGLELINE.0 | DT_END_ELLIPSIS.0 | DT_NOPREFIX.0);

pub(super) fn measure(font: HFONT, text: &str, width: i32) -> i32 {
    crate::layered::measure(font, &text.encode_utf16().collect::<Vec<_>>(), width, WRAP)
}

fn rect((x0, y0, x1, y1): (i32, i32, i32, i32)) -> RECT {
    RECT { left: x0, top: y0, right: x1, bottom: y1 }
}

fn ink(frame: &mut Frame, lines: &[Line], kind: Ink, (x0, y0): (i32, i32)) {
    for line in lines.iter().filter(|line| line.ink == kind) {
        let (a, b, c, d) = line.rect;
        frame.text(line.font, &line.text, RECT { left: x0 + a, top: y0 + b, right: x0 + c, bottom: y0 + d }, WRAP);
    }
}

pub(super) fn render(h: HWND, l: &Layout) {
    let s = |v| scale(h, v);
    let close = popup::close_rect(h);
    let placed = super::text(h, l.body.2 - l.body.0);
    let lines = &placed.lines;
    popup::paint(h, "dialog", |frame, a| {
        let (ht, head, mark) = (frame.h, s(HEAD), s(40));
        let (title, normal, bold, small, icons) = (
            a.font(s(18), FW_SEMIBOLD.0),
            a.font(s(13), FW_NORMAL.0),
            a.font(s(13), FW_SEMIBOLD.0),
            a.font(s(12), FW_NORMAL.0),
            a.icon_font(s(15)),
        );
        let d = a.dialog.as_ref()?;
        let ui = chrome::palette();
        let tx = s(22) + mark + s(14);
        let pill = d.spec.badge.as_ref().map(|b| {
            let x1 = close.left - s(10);
            (x1 - line_width(bold, &b.encode_utf16().collect::<Vec<_>>()) - s(24), s(15), x1, s(39))
        });
        let right = pill.map_or(close.left - s(10), |p| p.0 - s(10));
        let sub = d.spec.title != APP;
        let named = if sub { (tx, s(14), right, s(40)) } else { (tx, 0, right, head) };
        frame.text(title, &d.spec.title, rect(named), DT_LEFT | SINGLE);
        frame.text(icons, &g::CLOSE.to_string(), close, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        for (b, r) in d.spec.buttons.iter().zip(&l.buttons).skip(1) {
            frame.text(bold, &b.label, rect(*r), DT_CENTER | SINGLE);
        }
        if let (Some(p), Some(r)) = (&d.spec.progress, l.status) {
            frame.text(normal, &p.text, rect(r), DT_LEFT | SINGLE);
        }
        let (x0, y0, x1, y1) = l.body;
        let origin = (x0, y0 - d.scroll);
        frame.clip((x0, y0, x1, y1));
        frame.text(normal, &d.spec.body, RECT { left: x0, top: origin.1, right: x1, bottom: origin.1 + (1 << 20) }, WRAP);
        ink(frame, lines, Ink::Text, origin);
        frame.unclip();
        frame.flush();
        chrome::background(frame, (HEAD, d.close_hot), s);
        let mut c = frame.canvas();
        if let Some(px) = logo::pixels(mark) {
            blend(&mut c.px[..], c.w, &px, mark, (s(22), (head - mark) / 2));
        }
        if let Some(p) = pill {
            c.rrect(p, s(12) as f32, flat(premul(ACCENT, 70)), (0, ht), false);
            c.rrect(p, s(12) as f32, flat(premul(ui.link, 120)), (0, ht), true);
        }
        for &(a, b, cx, dy) in &placed.cards {
            let r = (x0 + a, origin.1 + b, x0 + cx, origin.1 + dy);
            c.rrect(r, s(8) as f32, flat(premul(ui.ink, 12)), (y0, y1), false);
            c.rrect(r, s(8) as f32, flat(premul(ui.ink, 22)), (y0, y1), true);
        }
        if let (Some(p), Some(bar)) = (&d.spec.progress, l.bar) {
            let r = s(4) as f32;
            c.rrect(bar, r, flat(premul(ui.ink, 30)), (0, ht), false);
            c.rrect(fill(bar, p.value, d.phase), r, flat(premul(ACCENT, 255)), (0, ht), false);
        }
        if let Some((ty, th)) = crate::domain::scroll::thumb(y1 - y0, l.text, d.scroll, s(24)) {
            c.rrect((x1 + s(8), y0 + ty, x1 + s(12), y0 + ty + th), s(2) as f32, flat(premul(ui.ink, 90)), (0, ht), false);
        }
        for (i, r) in l.buttons.iter().enumerate() {
            let hot = d.hover == Some(i);
            let fill = if i == 0 { premul(ACCENT, if hot { 255 } else { 220 }) } else { premul(ui.ink, if hot { 34 } else { 16 }) };
            c.rrect(*r, s(8) as f32, flat(fill), (0, ht), false);
            if i > 0 {
                c.rrect(*r, s(8) as f32, flat(premul(ui.ink, 50)), (0, ht), true);
            }
        }
        frame.text_layers(ui.text, ui.shade, s(1), (0, ht));
        if let (Some(b), Some(r)) = (d.spec.buttons.first(), l.buttons.first()) {
            frame.clear_mask();
            frame.text(bold, &b.label, rect(*r), DT_CENTER | SINGLE);
            frame.flush();
            frame.text_layers(ui.on_accent, ui.shade, s(1), (0, ht));
        }
        for (kind, color) in [(Ink::Accent, ui.link), (Ink::Dim, ui.dim)] {
            frame.clear_mask();
            frame.clip((x0, y0, x1, y1));
            ink(frame, lines, kind, origin);
            frame.unclip();
            match (kind, &d.spec.badge, pill) {
                (Ink::Accent, Some(b), Some(p)) => frame.text(bold, b, rect(p), DT_CENTER | SINGLE),
                (Ink::Dim, ..) if sub => frame.text(small, APP, rect((tx, s(38), right, s(58))), DT_LEFT | SINGLE),
                _ => {}
            }
            frame.flush();
            frame.text_layers(color, ui.shade, s(1), (0, ht));
        }
        if d.scroll < l.text - (y1 - y0) {
            let base = lerp(premul(ui.bg, 252), premul(ui.bg_end, 252), (y1 * 255 / ht.max(1)) as u32);
            frame.canvas().rrect((x0 - s(4), y1 - s(36), x1 + s(4), y1), 0., (0, base), (0, ht), false);
        }
        chrome::outline(frame, s);
        Some(())
    });
}
