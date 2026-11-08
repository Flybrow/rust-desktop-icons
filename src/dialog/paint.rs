use super::{
    layout::{Layout, fill},
    notes::{Ink, Line},
};
use crate::{
    layered::{ACCENT, Frame, WHITE, chrome, glyph as g, popup},
    render::{flat, premul},
    win::*,
};
use windows::Win32::{Foundation::*, Graphics::Gdi::*};

const WRAP: DRAW_TEXT_FORMAT = DRAW_TEXT_FORMAT(DT_LEFT.0 | DT_WORDBREAK.0 | DT_NOPREFIX.0 | DT_EXPANDTABS.0);

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
    let lines = super::text(h, l.body.2 - l.body.0).1;
    popup::paint(h, "dialog", |frame, a| {
        let (w, ht) = (frame.w, frame.h);
        let (title, normal, bold, icons) =
            (a.font(s(18), FW_SEMIBOLD.0), a.font(s(13), FW_NORMAL.0), a.font(s(13), FW_SEMIBOLD.0), a.icon_font(s(15)));
        let d = a.dialog.as_ref()?;
        let single = DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX;
        frame.text(title, &d.spec.title, RECT { left: s(22), top: 0, right: w - s(60), bottom: s(chrome::TOP) }, DT_LEFT | single);
        frame.text(icons, &g::CLOSE.to_string(), close, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        for (b, r) in d.spec.buttons.iter().zip(&l.buttons) {
            frame.text(bold, &b.label, rect(*r), DT_CENTER | single);
        }
        if let (Some(p), Some(r)) = (&d.spec.progress, l.status) {
            frame.text(normal, &p.text, rect(r), DT_LEFT | single);
        }
        let (x0, y0, x1, y1) = l.body;
        frame.clip((x0, y0, x1, y1));
        frame.text(normal, &d.spec.body, RECT { left: x0, top: y0 - d.scroll, right: x1, bottom: y0 - d.scroll + (1 << 20) }, WRAP);
        let origin = (x0, y0 - d.scroll);
        ink(frame, &lines, Ink::Text, origin);
        frame.flush();
        chrome::background(frame, d.close_hot, s);
        let mut c = frame.canvas();
        if let (Some(p), Some(bar)) = (&d.spec.progress, l.bar) {
            let r = s(4) as f32;
            c.rrect(bar, r, flat(premul(WHITE, 30)), (0, ht), false);
            c.rrect(fill(bar, p.value, d.phase), r, flat(premul(ACCENT, 255)), (0, ht), false);
        }
        let (bh, x1) = (y1 - y0, l.body.2);
        if l.text > bh {
            let th = (bh * bh / l.text).max(s(24));
            let ty = y0 + d.scroll * (bh - th) / (l.text - bh);
            c.rrect((x1 + s(8), ty, x1 + s(12), ty + th), s(2) as f32, flat(premul(WHITE, 90)), (0, ht), false);
        }
        for (i, r) in l.buttons.iter().enumerate() {
            let hot = d.hover == Some(i);
            let fill = if i == 0 { premul(ACCENT, if hot { 255 } else { 220 }) } else { premul(WHITE, if hot { 34 } else { 16 }) };
            c.rrect(*r, s(8) as f32, flat(fill), (0, ht), false);
            if i > 0 {
                c.rrect(*r, s(8) as f32, flat(premul(WHITE, 50)), (0, ht), true);
            }
        }
        frame.text_layers(chrome::TEXT, 0, s(1), (0, ht));
        for (kind, color) in
            [(Ink::Accent, chrome::LINK), (Ink::Dim, chrome::DIM)].into_iter().filter(|(k, _)| lines.iter().any(|l| l.ink == *k))
        {
            frame.clear_mask();
            ink(frame, &lines, kind, origin);
            frame.flush();
            frame.text_layers(color, 0, s(1), (0, ht));
        }
        chrome::outline(frame, s);
        Some(())
    });
}
