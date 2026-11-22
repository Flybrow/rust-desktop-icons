use crate::domain::columns::Mode;
use windows::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{DRAW_TEXT_FORMAT, DT_CENTER, DT_EDITCONTROL, DT_END_ELLIPSIS, DT_LEFT, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK},
};

pub(super) const LABEL: DRAW_TEXT_FORMAT = DRAW_TEXT_FORMAT(DT_CENTER.0 | DT_WORDBREAK.0 | DT_END_ELLIPSIS.0 | DT_EDITCONTROL.0);
pub(super) const LINE: DRAW_TEXT_FORMAT = DRAW_TEXT_FORMAT(DT_LEFT.0 | DT_VCENTER.0 | DT_SINGLELINE.0 | DT_END_ELLIPSIS.0);

pub(super) const TOP: i32 = 10;
const SIDE: i32 = 5;
const UNDER: i32 = 6;
pub(super) const PAD: i32 = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Cell {
    pub w: i32,
    pub h: i32,
    pub mode: Mode,
    pub name_w: i32,
    pub line: i32,
}

impl Cell {
    pub(super) fn square(size: i32) -> Self {
        Cell { w: size, h: size, mode: Mode::Icons, name_w: size, line: 0 }
    }

    pub(super) fn icons(self) -> bool {
        self.mode == Mode::Icons
    }

    pub(super) fn span(self, w: i32) -> i32 {
        crate::domain::grid::cols(w, self.w) * self.h
    }

    pub(super) fn flags(self) -> DRAW_TEXT_FORMAT {
        if self.icons() { LABEL } else { LINE }
    }
}

pub(super) fn icon_at((x, y): (i32, i32), c: Cell, icon: i32, s: impl Fn(i32) -> i32) -> (i32, i32) {
    if c.icons() { (x + (c.w - icon) / 2, y + s(TOP)) } else { (x + s(PAD), y + (c.h - icon) / 2) }
}

fn text_left(x: i32, icon: i32, s: impl Fn(i32) -> i32) -> i32 {
    x + s(2 * PAD) + icon
}

pub(super) fn split(x: i32, c: Cell) -> i32 {
    x + c.w * 11 / 20
}

pub(super) fn label_at((x, y): (i32, i32), c: Cell, icon: i32, s: impl Fn(i32) -> i32) -> RECT {
    let left = text_left(x, icon, &s);
    match c.mode {
        Mode::Icons => RECT { left: x + s(SIDE), top: y + s(TOP) + icon + s(UNDER), right: x + c.w - s(SIDE), bottom: y + c.h },
        Mode::List | Mode::Details => RECT { left, top: y, right: x + c.name_w - s(PAD), bottom: y + c.h },
        Mode::Content => RECT { left, top: y + c.h / 2 - c.line, right: split(x, c) - s(PAD), bottom: y + c.h / 2 },
        Mode::Tiles => {
            let top = y + (c.h - 3 * c.line) / 2;
            RECT { left, top, right: x + c.w - s(PAD), bottom: top + c.line }
        }
    }
}

pub(super) fn content(c: Cell, icon: i32, names: impl Iterator<Item = i32>, s: impl Fn(i32) -> i32) -> i32 {
    if !c.icons() {
        return c.h;
    }
    let room = c.h - s(TOP + UNDER) - icon;
    let label = names.max().unwrap_or(0).min(room);
    s(TOP) + icon + if label > 0 { s(UNDER) + label } else { 0 }
}

pub(super) fn tile((x, y): (i32, i32), c: Cell, icon: i32, label: i32, s: impl Fn(i32) -> i32) -> (i32, i32, i32, i32) {
    if !c.icons() {
        return (x + s(3), y + s(1), x + c.w - s(3), y + c.h - s(1));
    }
    let bottom = (content(c, icon, std::iter::once(label), &s) + s(TOP - 3)).min(c.h - s(1));
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
    fn list_rows_put_the_name_beside_the_icon() {
        let (s, c) = (|v: i32| v, Cell { w: 300, h: 40, mode: Mode::List, name_w: 300, line: 16 });
        assert_eq!(icon_at((10, 100), c, 24, s), (18, 108));
        let r = label_at((10, 100), c, 24, s);
        assert_eq!((r.left, r.top, r.right, r.bottom), (10 + 16 + 24, 100, 302, 140));
        assert_eq!(content(c, 24, [99].into_iter(), s), 40);
        assert_eq!(tile((0, 0), c, 24, 0, s), (3, 1, 297, 39));
        assert_eq!((c.flags(), Cell::square(9).flags()), (LINE, LABEL));
        assert_eq!((c.span(300), c.span(900), Cell::square(100).span(350)), (40, 120, 300));
    }

    #[test]
    fn rich_modes_lay_text_beside_the_icon() {
        let s = |v: i32| v;
        let d = Cell { w: 600, h: 24, mode: Mode::Details, name_w: 250, line: 16 };
        let r = label_at((0, 100), d, 16, s);
        assert_eq!((r.left, r.top, r.right, r.bottom), (32, 100, 242, 124));
        let t = Cell { w: 260, h: 64, mode: Mode::Tiles, name_w: 260, line: 16 };
        let r = label_at((10, 0), t, 48, s);
        assert_eq!((r.left, r.top, r.right, r.bottom), (74, 8, 262, 24));
        assert_eq!(icon_at((10, 0), t, 48, s), (18, 8));
        let c = Cell { w: 400, h: 48, mode: Mode::Content, name_w: 400, line: 16 };
        let r = label_at((0, 0), c, 32, s);
        assert_eq!((r.left, r.top, r.right, r.bottom, split(0, c)), (48, 8, 212, 24, 220));
        assert_eq!((content(t, 48, [99].into_iter(), s), tile((0, 0), t, 48, 0, s)), (64, (3, 1, 257, 63)));
    }
}
