use super::header::{room, segment, strip, title_align as align};
use crate::{
    domain::{Fence, color, vertical},
    layered::Frame,
    render::{Canvas, flat, premul},
};
use windows::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{DRAW_TEXT_FORMAT, DT_CENTER, DT_END_ELLIPSIS, DT_SINGLELINE, DT_VCENTER, HFONT},
};

fn tabs(f: &Fence, len: i32) -> Vec<(i32, i32)> {
    let n = f.tabs.len();
    if n > 1 { (0..n).map(|i| segment(len, n, i)).collect() } else { vec![] }
}

pub(super) fn text(
    bar: &Frame,
    font: HFONT,
    (f, hint): (&Fence, Option<&str>),
    loupe: Option<bool>,
    upright: bool,
    s: &impl Fn(i32) -> i32,
) {
    let (len, t) = (bar.w, bar.h);
    let room = room(len, t, f, loupe);
    let segs = tabs(f, len);
    let whole = (s(16), room(len, len - s(16)));
    let lines: Vec<(HFONT, &str, (i32, i32), DRAW_TEXT_FORMAT)> = match hint {
        Some(h) => vec![(font, h, whole, align(f, hint))],
        None if segs.is_empty() => vec![(font, &f.tabs[0].title, whole, align(f, None))],
        None => (f.tabs.iter().zip(segs))
            .map(|(tab, (x0, x1))| (font, tab.title.as_str(), (x0 + s(10), room(x1, x1 - s(10))), DT_CENTER))
            .collect(),
    };
    for (font, title, (l, r), a) in lines.into_iter().filter(|l| !(upright && vertical::has_ideographic(l.1))) {
        bar.text(font, title, RECT { left: l, top: 0, right: r, bottom: t }, DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | a);
    }
}

pub(super) fn shapes(c: &mut Canvas, f: &Fence, (active, plain): (Option<usize>, bool), edge: (bool, i32), s: &impl Fn(i32) -> i32) {
    let (cy, t, r) = (c.h / 2, c.h, f.look.corner.px());
    if let Some(col) = f.look.bar_color {
        c.rrect(strip((c.w, t), edge, s), s(r) as f32, flat(premul(col, (f.look.alpha as u32 * 4 / 5) as u8)), (0, t), false);
    }
    let Some((x0, x1)) = active.filter(|_| plain).and_then(|i| tabs(f, c.w).get(i).copied()) else { return };
    let fill = if color::ink(f.look.bar_color.unwrap_or(f.look.color)) == 0 { premul(0, 16) } else { premul(0xFF_FFFF, 26) };
    c.rrect((x0 + s(4), cy - s(13), x1 - s(4), cy + s(13)), s(r.min(8)) as f32, flat(fill), (0, t), false);
}
