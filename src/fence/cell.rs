use windows::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{DRAW_TEXT_FORMAT, DT_CENTER, DT_EDITCONTROL, DT_END_ELLIPSIS, DT_LEFT, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK},
};

pub(super) const LABEL: DRAW_TEXT_FORMAT = DRAW_TEXT_FORMAT(DT_CENTER.0 | DT_WORDBREAK.0 | DT_END_ELLIPSIS.0 | DT_EDITCONTROL.0);
const LINE: DRAW_TEXT_FORMAT = DRAW_TEXT_FORMAT(DT_LEFT.0 | DT_VCENTER.0 | DT_SINGLELINE.0 | DT_END_ELLIPSIS.0);

const TOP: i32 = 10;
const SIDE: i32 = 5;
const UNDER: i32 = 6;
const TIGHT: (i32, i32) = (2, 3);
const PAD: i32 = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Cell {
    pub w: i32,
    pub h: i32,
    pub list: bool,
    pub top: i32,
    pub under: i32,
}

impl Cell {
    pub(super) fn square(size: i32) -> Self {
        Cell { w: size, h: size, list: false, top: TOP, under: UNDER }
    }

    pub(super) fn of(look: &crate::domain::Look, gw: i32, ch: i32) -> Self {
        let (top, under) = if look.style == crate::domain::Style::Fluent { TIGHT } else { (TOP, UNDER) };
        Cell { w: if look.list { gw } else { ch }, h: ch, list: look.list, top, under }
    }

    pub(super) fn span(self, w: i32) -> i32 {
        if self.list { self.h } else { w }
    }

    pub(super) fn flags(self) -> DRAW_TEXT_FORMAT {
        if self.list { LINE } else { LABEL }
    }
}

pub(super) fn icon_at((x, y): (i32, i32), c: Cell, icon: i32, s: impl Fn(i32) -> i32) -> (i32, i32) {
    if c.list { (x + s(PAD), y + (c.h - icon) / 2) } else { (x + (c.w - icon) / 2, y + s(c.top)) }
}

pub(super) fn label_at((x, y): (i32, i32), c: Cell, icon: i32, s: impl Fn(i32) -> i32) -> RECT {
    match c.list {
        true => RECT { left: x + s(2 * PAD) + icon, top: y, right: x + c.w - s(PAD), bottom: y + c.h },
        false => RECT { left: x + s(SIDE), top: y + s(c.top) + icon + s(c.under), right: x + c.w - s(SIDE), bottom: y + c.h },
    }
}

pub(super) fn content(c: Cell, icon: i32, names: impl Iterator<Item = i32>, s: impl Fn(i32) -> i32) -> i32 {
    if c.list {
        return c.h;
    }
    let room = c.h - s(c.top + c.under) - icon;
    let label = names.max().unwrap_or(0).min(room);
    s(c.top) + icon + if label > 0 { s(c.under) + label } else { 0 }
}

pub(super) fn tile((x, y): (i32, i32), c: Cell, icon: i32, label: i32, s: impl Fn(i32) -> i32) -> (i32, i32, i32, i32) {
    if c.list {
        return (x + s(3), y + s(1), x + c.w - s(3), y + c.h - s(1));
    }
    let bottom = (content(c, icon, std::iter::once(label), &s) + s(c.top - 3)).min(c.h - s(1));
    (x + s(SIDE), y + s(3), x + c.w - s(SIDE), y + bottom)
}

pub(super) fn label_width(c: Cell, s: impl Fn(i32) -> i32) -> i32 {
    c.w - s(2 * SIDE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_row_height_follows_its_content() {
        let (s, c) = (|v: i32| v * 2, Cell::square(208));
        assert_eq!(content(c, 96, [0, 0].into_iter(), s), 20 + 96);
        assert_eq!(content(c, 96, [32, 16].into_iter(), s), 20 + 96 + 12 + 32);
        assert_eq!(content(c, 96, [500].into_iter(), s), 208);
        assert_eq!(content(c, 96, std::iter::empty(), s), 20 + 96);
        assert_eq!(label_width(c, s), 188);
        assert_eq!(tile((100, 50), c, 96, 32, s), (110, 56, 298, 50 + 160 + 14));
        assert_eq!(tile((100, 50), c, 96, 500, s), (110, 56, 298, 50 + 206));
        assert_eq!(tile((0, 0), c, 96, 0, s).3, 20 + 96 + 14);
    }

    #[test]
    fn fluent_cells_are_tight_like_fences_7() {
        let mut look = crate::domain::Look { style: crate::domain::Style::Fluent, ..Default::default() };
        let c = Cell::of(&look, 455, 91);
        assert_eq!((c.w, c.h, c.top, c.under), (91, 91, 2, 3));
        assert_eq!(icon_at((0, 0), c, 48, |v| v), (21, 2));
        assert_eq!(label_at((0, 0), c, 48, |v| v).top, 53);
        look.list = true;
        assert_eq!(Cell::of(&look, 455, 44).w, 455);
    }

    #[test]
    fn list_rows_put_the_name_beside_the_icon() {
        let (s, c) = (|v: i32| v, Cell { w: 300, h: 40, list: true, ..Cell::square(40) });
        assert_eq!(icon_at((10, 100), c, 24, s), (18, 108));
        let r = label_at((10, 100), c, 24, s);
        assert_eq!((r.left, r.top, r.right, r.bottom), (10 + 16 + 24, 100, 302, 140));
        assert_eq!(content(c, 24, [99].into_iter(), s), 40);
        assert_eq!(tile((0, 0), c, 24, 0, s), (3, 1, 297, 39));
        assert_eq!((c.flags(), Cell::square(9).flags()), (LINE, LABEL));
    }
}
