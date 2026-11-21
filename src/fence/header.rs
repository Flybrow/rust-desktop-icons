use crate::{
    domain::{Fence, color, grid::Align, side::Geo, vertical},
    layered::{Frame, measure},
    render::{Canvas, blit, flat, premul, rotate_cw},
};
use windows::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_RIGHT, DT_SINGLELINE, DT_VCENTER, HFONT},
};

fn segment(w: i32, n: usize, i: usize) -> (i32, i32) {
    let n = n.max(1) as i32;
    (i as i32 * w / n, (i as i32 + 1) * w / n)
}

fn text_colors(f: &Fence) -> (u32, u32) {
    color::text_on(f.look.bar_color.unwrap_or(f.look.color))
}

fn text(frame: &Frame, font: HFONT, (f, hint): (&Fence, Option<&str>), t: i32, pad: i32, upright: bool) {
    let (w, n) = (frame.w, f.tabs.len());
    let skip = |title: &str| upright && vertical::has_ideographic(title);
    let flags = DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS;
    if n == 1 || hint.is_some() {
        let title = hint.unwrap_or(&f.tabs[0].title);
        if skip(title) {
            return;
        }
        let align = match f.look.title_align {
            _ if hint.is_some_and(|h| h.starts_with('←')) => DT_LEFT,
            Align::Left => DT_LEFT,
            Align::Center => DT_CENTER,
            Align::Right => DT_RIGHT,
        };
        return frame.text(font, title, RECT { left: t / 2, top: 0, right: w - t / 2, bottom: t }, flags | align);
    }
    let flags = flags | DT_CENTER;
    for (i, tab) in f.tabs.iter().enumerate() {
        if skip(&tab.title) {
            continue;
        }
        let (x0, x1) = segment(w, n, i);
        frame.text(font, &tab.title, RECT { left: x0 + pad, top: 0, right: x1 - pad, bottom: t }, flags);
    }
}

fn shapes(c: &mut Canvas, f: &Fence, (active, tabs): (Option<usize>, bool), flipped: bool, s: impl Fn(i32) -> i32) {
    let (w, t, n, body) = (c.w, c.h, f.tabs.len(), f.look.alpha);
    let base = f.look.bar_color.unwrap_or(f.look.color);
    let (tone, ink) = (|k, a: u8| premul(color::shade(base, k), a), color::ink(base));
    let (shape, line) = if flipped { ((0, -s(16), w, t), 0) } else { ((0, 0, w, t + s(16)), t - 1) };
    let r = f.look.corner.px();
    c.rrect(shape, s(r) as f32, (tone(-20, body.saturating_add(50)), tone(-34, body.saturating_add(60))), (0, t), false);
    c.rrect((s(12), line, w - s(12), line + 1), 0., flat(premul(ink, 34)), (0, t), false);
    if n > 1 && tabs {
        if let Some(i) = active {
            let (x0, x1) = segment(w, n, i.min(n - 1));
            c.rrect((x0 + s(4), s(4), x1 - s(4), t - s(4)), s(r.min(6)) as f32, flat(premul(ink, 38)), (0, t), false);
        }
        for i in 1..n {
            let x = segment(w, n, i).0;
            c.rrect((x, s(9), x + 1, t - s(9)), 0., flat(premul(ink, 40)), (0, t), false);
        }
    }
}

fn columns(len: i32, t: i32, (f, hint): (&Fence, Option<&str>), pad: i32) -> Vec<(String, (i32, i32), Align)> {
    let n = f.tabs.len();
    match hint {
        Some(h) if vertical::has_ideographic(h) => {
            let back = h.starts_with('←');
            vec![(h.replace('←', "↑"), (t / 2, len - t / 2), if back { Align::Left } else { f.look.title_align })]
        }
        Some(_) => vec![],
        None if n == 1 => {
            let title = &f.tabs[0].title;
            if vertical::has_ideographic(title) { vec![(title.clone(), (t / 2, len - t / 2), f.look.title_align)] } else { vec![] }
        }
        None => (f.tabs.iter().enumerate())
            .filter(|(_, tab)| vertical::has_ideographic(&tab.title))
            .map(|(i, tab)| {
                let (y0, y1) = segment(len, n, i);
                (tab.title.clone(), (y0 + pad, y1 - pad), Align::Center)
            })
            .collect(),
    }
}

fn upright(turned: &[u32], (t, len): (i32, i32), font: HFONT, (f, hint): (&Fence, Option<&str>), s: &impl Fn(i32) -> i32) -> Option<Frame> {
    let columns = columns(len, t, (f, hint), s(8));
    if columns.is_empty() {
        return None;
    }
    let mut col = Frame::new(t, len)?;
    col.canvas.px().copy_from_slice(turned);
    let probe: Vec<u16> = "国Ag".encode_utf16().collect();
    let line = measure(font, &probe, t, DT_SINGLELINE | DT_CENTER).max(1);
    for (title, range, align) in columns {
        for (ch, y) in vertical::column(&title, line, range, align).into_iter().filter(|(c, _)| *c != ' ') {
            col.text(font, &ch.to_string(), RECT { left: 0, top: y, right: t, bottom: y + line }, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        }
    }
    col.flush();
    let (fg, shade) = text_colors(f);
    col.text_layers(fg, shade, s(2), (0, len));
    Some(col)
}

pub(super) fn paint(
    main: &mut Frame,
    g: &Geo,
    font: HFONT,
    (f, hint): (&Fence, Option<&str>),
    active: Option<usize>,
    s: impl Fn(i32) -> i32,
) {
    let (fg, shade) = text_colors(f);
    let (len, t) = (g.length(), g.bar);
    let side_bar = g.side.vertical();
    let Some(mut bar) = Frame::new(len, t) else { return };
    text(&bar, font, (f, hint), t, s(8), side_bar);
    bar.flush();
    shapes(&mut bar.canvas(), f, (active, hint.is_none()), g.side.flipped(), &s);
    bar.text_layers(fg, shade, s(2), (0, t));
    let (x0, y0, _, _) = g.bar_rect();
    if !side_bar {
        let c = main.canvas();
        return blit(c.px, c.w, bar.canvas.px(), len, (x0, y0));
    }
    let turned = rotate_cw(bar.canvas.px(), len, t);
    let mut col = upright(&turned, (t, len), font, (f, hint), &s);
    let c = main.canvas();
    blit(c.px, c.w, col.as_mut().map_or(&turned[..], |col| &*col.canvas.px()), t, (x0, y0));
}
