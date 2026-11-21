use crate::{
    domain::{Fence, color, grid::Align, side::Geo, vertical},
    layered::{Frame, WHITE, measure},
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
    let tone = |k, a: u8| premul(color::shade(f.look.bar_color.unwrap_or(f.look.color), k), a);
    let (shape, line) = if flipped { ((0, -s(16), w, t), 0) } else { ((0, 0, w, t + s(16)), t - 1) };
    c.rrect(shape, s(8) as f32, (tone(-20, body.saturating_add(50)), tone(-34, body.saturating_add(60))), (0, t), false);
    c.rrect((s(12), line, w - s(12), line + 1), 0., flat(premul(WHITE, 34)), (0, t), false);
    if n > 1 && tabs {
        if let Some(i) = active {
            let (x0, x1) = segment(w, n, i.min(n - 1));
            c.rrect((x0 + s(4), s(4), x1 - s(4), t - s(4)), s(6) as f32, flat(premul(WHITE, 38)), (0, t), false);
        }
        for i in 1..n {
            let x = segment(w, n, i).0;
            c.rrect((x, s(9), x + 1, t - s(9)), 0., flat(premul(WHITE, 40)), (0, t), false);
        }
    }
}

fn upright(main: &mut Frame, g: &Geo, font: HFONT, (f, hint): (&Fence, Option<&str>), s: impl Fn(i32) -> i32) {
    let wide = match hint {
        Some(h) => vertical::has_ideographic(h),
        None => f.tabs.iter().any(|t| vertical::has_ideographic(&t.title)),
    };
    if !wide {
        return;
    }
    let (x0, y0, x1, y1) = g.bar_rect();
    let (w, len, n, pad, inset) = (x1 - x0, y1 - y0, f.tabs.len(), s(8), g.bar / 2);
    let probe: Vec<u16> = "国Ag".encode_utf16().collect();
    let line = measure(font, &probe, w, DT_SINGLELINE | DT_CENTER).max(1);
    let mut columns = vec![];
    match hint {
        Some(h) => {
            let back = h.starts_with('←');
            let label = if back { h.replace('←', "↑") } else { h.to_string() };
            columns.push((label, (y0 + inset, y1 - inset), if back { Align::Left } else { f.look.title_align }));
        }
        None => {
            for (i, tab) in f.tabs.iter().enumerate() {
                if !vertical::has_ideographic(&tab.title) {
                    continue;
                }
                let (from, to) = if n == 1 {
                    (y0 + inset, y1 - inset)
                } else {
                    (y0 + i as i32 * len / n as i32 + pad, y0 + (i as i32 + 1) * len / n as i32 - pad)
                };
                let align = if n == 1 { f.look.title_align } else { Align::Center };
                columns.push((tab.title.clone(), (from, to), align));
            }
        }
    }
    let mut cells = vec![];
    for (title, (from, to), align) in columns {
        let mut chars: Vec<char> = title.chars().filter(|c| !c.is_whitespace()).collect();
        let ys = vertical::stack(chars.len(), line, (from, to), align);
        if ys.is_empty() {
            continue;
        }
        if ys.len() < chars.len() {
            chars.truncate(ys.len() - 1);
            chars.push('…');
        }
        cells.extend(chars.into_iter().zip(ys));
    }
    if cells.is_empty() {
        return;
    }
    main.unclip();
    main.clear_mask();
    main.clip((x0, y0, x1, y1));
    for (ch, y) in cells {
        let r = RECT { left: x0, top: y, right: x1, bottom: y + line };
        main.text(font, &ch.to_string(), r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
    }
    main.flush();
    let (fg, shade) = text_colors(f);
    main.text_layers(fg, shade, s(2), (y0, y1));
    main.unclip();
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
    {
        let c = main.canvas();
        if side_bar {
            blit(c.px, c.w, &rotate_cw(bar.canvas.px(), len, t), t, (x0, y0));
        } else {
            blit(c.px, c.w, bar.canvas.px(), len, (x0, y0));
        }
    }
    if side_bar {
        upright(main, g, font, (f, hint), s);
    }
}
