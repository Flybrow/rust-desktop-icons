use super::layout::{Layout, fill};
use crate::{
    app::with,
    layered::{ACCENT, Frame, WHITE, chrome, glyph as g},
    render::{flat, premul},
    win::*,
};
use windows::Win32::{Foundation::*, Graphics::Gdi::*};

const WRAP: DRAW_TEXT_FORMAT = DRAW_TEXT_FORMAT(DT_LEFT.0 | DT_WORDBREAK.0 | DT_NOPREFIX.0 | DT_EXPANDTABS.0);

pub(super) fn measure(font: HFONT, text: &str, width: i32) -> i32 {
    if text.is_empty() {
        return 0;
    }
    let mut w: Vec<u16> = text.encode_utf16().collect();
    let mut r = RECT { left: 0, top: 0, right: width, bottom: 0 };
    unsafe {
        let dc = CreateCompatibleDC(None);
        let old = SelectObject(dc, font.into());
        DrawTextW(dc, &mut w, &mut r, WRAP | DT_CALCRECT);
        SelectObject(dc, old);
        let _ = DeleteDC(dc);
    }
    r.bottom
}

fn rect((x0, y0, x1, y1): (i32, i32, i32, i32)) -> RECT {
    RECT { left: x0, top: y0, right: x1, bottom: y1 }
}

pub(super) fn render(h: HWND, l: &Layout) {
    let wr = window_rect(h);
    let (w, ht) = (wr.right - wr.left, wr.bottom - wr.top);
    let s = |v| scale(h, v);
    let Some(mut frame) = Frame::new(w, ht) else { return };
    let close = chrome::close_rect(w, s);
    let drawn = with(|a| {
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
        frame.flush();
        chrome::background(&mut frame, d.close_hot, s);
        let mut c = frame.canvas();
        if let (Some(p), Some(bar)) = (&d.spec.progress, l.bar) {
            let r = s(4) as f32;
            c.rrect(bar, r, flat(premul(WHITE, 30)), (0, ht), false);
            c.rrect(fill(bar, p.value, d.phase), r, flat(premul(ACCENT, 255)), (0, ht), false);
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
        chrome::outline(&mut frame, s);
        #[cfg(debug_assertions)]
        frame.dump("dialog");
        Some(())
    });
    if drawn.flatten().is_some() {
        frame.present(h, (wr.left, wr.top), 255);
    }
}
