use super::{cell, label_font, layout, metrics, view};
use crate::{
    app::with,
    layered::{Dib, Frame, WHITE},
    render::{opaque_if_flat, straight},
    shell::{self, DragImage},
    win::*,
};
use windows::Win32::Foundation::HWND;

pub(super) fn image(h: HWND, index: usize, (cx, cy): (i32, i32)) -> Option<DragImage> {
    let s = |v| scale(h, v);
    let icon = metrics(h).1;
    let g = layout::geo_now(h);
    let shape = super::shape(h, g.grid_w());
    let (w, ht) = (shape.w, shape.h);
    let (mut frame, mut icons) = (Frame::new(w, ht)?, Dib::new(w, ht)?);
    with(|a| {
        let font = label_font(a, h);
        let v = a.view(h)?;
        let item = v.items.get_mut(index)?;
        frame.text_w(font, &mut item.name, &mut cell::label_at((0, 0), shape, icon, s), shape.flags());
        frame.target(&icons);
        shell::draw_icon(frame.dc(), item.icon, cell::icon_at((0, 0), shape, icon, s), icon);
        frame.target(&frame.mask);
        Some(())
    })
    .flatten()?;
    frame.flush();
    let (ix, iy) = cell::icon_at((0, 0), shape, icon, s);
    opaque_if_flat(icons.px(), w, (ix, iy, ix + icon, iy + icon));
    let px = icons.px().to_vec();
    frame.canvas().layer(&px, (0, 0), None, (0, ht));
    frame.text_layers(WHITE, 0, s(2), (0, ht));
    let (x, y) = with(|a| layout::flow(a, h, g.grid_w(), shape)).flatten()?.origin(index);
    let (x, y) = g.cell((x, y - view(h, |v| v.scroll).unwrap_or(0)), (w, ht));
    let offset = (cx - x, cy - y);
    straight(frame.canvas.px());
    Some(DragImage { bitmap: frame.export_bitmap()?, size: (w, ht), offset })
}
