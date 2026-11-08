use windows::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{DRAW_TEXT_FORMAT, DT_CENTER, DT_EDITCONTROL, DT_END_ELLIPSIS, DT_WORDBREAK},
};

pub(super) const LABEL: DRAW_TEXT_FORMAT = DRAW_TEXT_FORMAT(DT_CENTER.0 | DT_WORDBREAK.0 | DT_END_ELLIPSIS.0 | DT_EDITCONTROL.0);

pub(super) const TOP: i32 = 10;
const SIDE: i32 = 5;
const UNDER: i32 = 6;

pub(super) fn icon_at((x, y): (i32, i32), cell: i32, icon: i32, s: impl Fn(i32) -> i32) -> (i32, i32) {
    (x + (cell - icon) / 2, y + s(TOP))
}

pub(super) fn label_at((x, y): (i32, i32), cell: i32, icon: i32, s: impl Fn(i32) -> i32) -> RECT {
    RECT { left: x + s(SIDE), top: y + s(TOP) + icon + s(UNDER), right: x + cell - s(SIDE), bottom: y + cell }
}

pub(super) fn content(cell: i32, icon: i32, names: impl Iterator<Item = i32>, s: impl Fn(i32) -> i32) -> i32 {
    let room = cell - s(TOP + UNDER) - icon;
    let label = names.max().unwrap_or(0).min(room);
    s(TOP) + icon + if label > 0 { s(UNDER) + label } else { 0 }
}

pub(super) fn tile((x, y): (i32, i32), cell: i32, icon: i32, label: i32, s: impl Fn(i32) -> i32) -> (i32, i32, i32, i32) {
    let bottom = (content(cell, icon, std::iter::once(label), &s) + s(TOP - 3)).min(cell - s(1));
    (x + s(SIDE), y + s(3), x + cell - s(SIDE), y + bottom)
}

pub(super) fn label_width(cell: i32, s: impl Fn(i32) -> i32) -> i32 {
    cell - s(2 * SIDE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_row_height_follows_its_content() {
        let s = |v: i32| v * 2;
        assert_eq!(content(208, 96, [0, 0].into_iter(), s), 20 + 96);
        assert_eq!(content(208, 96, [32, 16].into_iter(), s), 20 + 96 + 12 + 32);
        assert_eq!(content(208, 96, [500].into_iter(), s), 208);
        assert_eq!(content(208, 96, std::iter::empty(), s), 20 + 96);
        assert_eq!(label_width(208, s), 188);
        assert_eq!(tile((100, 50), 208, 96, 32, s), (110, 56, 298, 50 + 160 + 14));
        assert_eq!(tile((100, 50), 208, 96, 500, s), (110, 56, 298, 50 + 206));
        assert_eq!(tile((0, 0), 208, 96, 0, s).3, 20 + 96 + 14);
    }
}
