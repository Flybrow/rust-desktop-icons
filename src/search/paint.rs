use super::{ROW, STATE};
use crate::{
    i18n::T,
    layered::{ACCENT, Dib, chrome, glyph as g, popup},
    render::{flat, opaque_if_flat, premul},
    shell,
    win::*,
};
use windows::Win32::{Foundation::*, Graphics::Gdi::*};

const ICON: i32 = 32;

pub(super) fn render(h: HWND) {
    let s = |v| scale(h, v);
    let left = DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS;
    popup::paint(h, "search", |frame, a| {
        let (w, ht) = (frame.w, frame.h);
        let mut icons = Dib::new(w, ht)?;
        let (big, normal, small, glyphs) =
            (a.font(s(18), FW_NORMAL.0), a.font(s(14), FW_NORMAL.0), a.font(s(12), FW_NORMAL.0), a.icon_font(s(18)));
        let (hint, none) = (a.t(T::SearchHint), a.t(T::NoResults));
        STATE.with(|st| {
            let st = st.try_borrow().ok()?;
            let st = st.as_ref()?;
            let (top, pad, icon) = (s(chrome::TOP), s(16), s(ICON));
            let x_text = pad + icon + s(14);
            let input = RECT { left: s(54), top: 0, right: w - pad, bottom: top };
            let row = |i: usize| top + s(6) + i as i32 * s(ROW);
            if !st.query.is_empty() {
                frame.text(big, &st.query, input, left);
            }
            for (i, &k) in st.shown.iter().enumerate() {
                let y = row(i);
                frame.text(normal, &st.all[k].name, RECT { left: x_text, top: y + s(4), right: w - pad, bottom: y + s(26) }, left);
            }
            frame.target(&icons);
            for (i, &k) in st.shown.iter().enumerate() {
                shell::draw_icon(frame.dc(), st.all[k].icon, (pad + s(4), row(i) + (s(ROW) - icon) / 2), icon);
            }
            frame.flush();
            for i in 0..st.shown.len() {
                let (x, y) = (pad + s(4), row(i) + (s(ROW) - icon) / 2);
                opaque_if_flat(icons.px(), w, (x, y, x + icon, y + icon));
            }
            chrome::background(frame, false, s);
            let mut c = frame.canvas();
            if !st.shown.is_empty() {
                let y = row(st.sel);
                c.rrect((s(8), y, w - s(8), y + s(ROW)), s(6) as f32, flat(premul(ACCENT, 70)), (0, ht), false);
            }
            c.layer(icons.px(), (0, 0), None, (0, ht));
            frame.text_layers(chrome::TEXT, 0, s(1), (0, ht));
            frame.clear_mask();
            frame.target(&frame.mask);
            frame.text(
                glyphs,
                &g::SEARCH.to_string(),
                RECT { left: s(18), top: 0, right: s(46), bottom: top },
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );
            if st.query.is_empty() {
                frame.text(big, hint, input, left);
            } else if st.shown.is_empty() {
                frame.text(normal, none, RECT { left: pad, top: row(0), right: w - pad, bottom: row(1) }, left);
            }
            for (i, &k) in st.shown.iter().enumerate() {
                let y = row(i);
                frame.text(small, &st.all[k].place, RECT { left: x_text, top: y + s(25), right: w - pad, bottom: y + s(44) }, left);
            }
            frame.flush();
            frame.text_layers(chrome::LINK, 0, s(1), (0, ht));
            chrome::outline(frame, s);
            Some(())
        })
    });
}
