use super::{actions, update};
use crate::{
    app::App,
    domain::{Fence, grid::Align, icons::LabelSize, inherit::Field},
    i18n::{self, T},
    layered::glyph as g,
    win::*,
};
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::HMENU};

const ALIGN: usize = 200;
const SIZE: usize = 203;
const AUTO: usize = 206;
const COLOR: usize = 207;
const HOVER: usize = 208;

pub(super) fn submenu(a: &mut App, m: HMENU, f: &Fence) {
    let (bar, align, size, color) = (menu(), menu(), menu(), menu());
    for (k, al) in Align::ALL.iter().enumerate() {
        item(align, ALIGN + k, a.t(i18n::align(*al)), f.look.title_align == *al);
    }
    for (k, z) in LabelSize::ALL.iter().enumerate() {
        item(size, SIZE + k, a.t(i18n::label_size(*z)), f.look.title_size == *z);
    }
    item(color, AUTO, a.t(T::Auto), f.look.bar_color.is_none());
    item(color, COLOR, a.t(T::Color), f.look.bar_color.is_some());
    a.sub(bar, T::TitleAlign, align, g::ALIGN);
    a.sub(bar, T::TitleSize, size, g::FONT);
    a.sub(bar, T::BarColor, color, g::COLOR);
    a.entry(bar, HOVER, T::BarHover, f.look.bar_hover, g::HIDE);
    a.sub(m, T::TitleBar, bar, g::TAB);
}

pub(super) fn handle(h: HWND, id: usize) -> bool {
    match id {
        k if (ALIGN..ALIGN + 3).contains(&k) => update(h, |f| f.set(Field::TitleAlign, |l| l.title_align = Align::ALL[k - ALIGN])),
        k if (SIZE..SIZE + 3).contains(&k) => update(h, |f| f.set(Field::TitleSize, |l| l.title_size = LabelSize::ALL[k - SIZE])),
        AUTO => update(h, |f| f.set(Field::BarColor, |l| l.bar_color = None)),
        COLOR => actions::bar_color(h),
        HOVER => update(h, |f| f.set(Field::BarHover, |l| l.bar_hover ^= true)),
        _ => return false,
    }
    true
}
