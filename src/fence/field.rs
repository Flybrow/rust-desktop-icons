use crate::{
    layered::{Frame, glyph, line_width},
    render::{flat, premul},
};
use windows::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_SINGLELINE, DT_VCENTER, HFONT},
};

pub struct Field {
    pub text: String,
    pub hint: bool,
    pub caret: bool,
}

struct Spots {
    rect: (i32, i32, i32, i32),
    text: (i32, i32),
}

fn spots(len: i32, t: i32, s: &impl Fn(i32) -> i32) -> Spots {
    let rect = (s(6), s(5), len - s(6), t - s(5));
    Spots { rect, text: (rect.0 + s(30), len - t) }
}

pub(super) fn text(bar: &Frame, (font, glyphs): (HFONT, HFONT), f: &Field, t: i32, s: &impl Fn(i32) -> i32) {
    let (len, p) = (bar.w, spots(bar.w, t, s));
    let one = DT_CENTER | DT_VCENTER | DT_SINGLELINE;
    let icon = RECT { left: p.rect.0 + s(6), top: 0, right: p.rect.0 + s(26), bottom: t };
    bar.text(glyphs, &glyph::SEARCH.to_string(), icon, one);
    bar.text(glyphs, &glyph::CLOSE.to_string(), RECT { left: len - t, top: 0, right: len - s(6), bottom: t }, one);
    let r = RECT { left: p.text.0, top: 0, right: p.text.1, bottom: t };
    bar.text(font, &f.text, r, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS);
}

pub(super) fn dim_hint(bar: &mut Frame, f: &Field, t: i32, s: &impl Fn(i32) -> i32) {
    if !f.hint {
        return;
    }
    let (w, (x0, x1)) = (bar.w, spots(bar.w, t, s).text);
    for (i, p) in bar.mask.px().iter_mut().enumerate() {
        if (x0..x1).contains(&(i as i32 % w)) {
            *p = (*p & 0xFF) * 120 / 255;
        }
    }
}

pub(super) fn shapes(bar: &mut Frame, ink: u32, t: i32, s: &impl Fn(i32) -> i32) {
    let rect = spots(bar.w, t, s).rect;
    bar.canvas().rrect(rect, s(6) as f32, flat(premul(ink, 16)), (0, t), false);
}

pub(super) fn caret(bar: &mut Frame, font: HFONT, f: &Field, fg: u32, t: i32, s: &impl Fn(i32) -> i32) {
    if !f.caret {
        return;
    }
    let (x0, x1) = spots(bar.w, t, s).text;
    let x = if f.hint { x0 - s(3) } else { (x0 + line_width(font, &f.text.encode_utf16().collect::<Vec<_>>()) + s(1)).min(x1) };
    let (y0, y1) = (t * 3 / 10, t - t * 3 / 10);
    bar.canvas().rrect((x, y0, x + s(1).max(1), y1), 0., flat(premul(fg, 255)), (0, t), false);
}
