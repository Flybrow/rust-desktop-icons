use serde::{Deserialize, Serialize};

pub const SIZES: [i32; 8] = [24, 32, 40, 48, 64, 80, 96, 128];
pub const DEFAULT: i32 = 48;

pub fn nearest(px: i32) -> i32 {
    *SIZES.iter().min_by_key(|s| (*s - px).abs()).unwrap_or(&DEFAULT)
}

pub fn step(px: i32, up: bool) -> i32 {
    let i = SIZES.iter().position(|&s| s == nearest(px)).unwrap_or(3);
    SIZES[if up { (i + 1).min(SIZES.len() - 1) } else { i.saturating_sub(1) }]
}

pub fn label(px: i32) -> String {
    format!("{px} px")
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelSize {
    Small,
    #[default]
    Normal,
    Large,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Spacing {
    Compact,
    #[default]
    Normal,
    Wide,
}

impl LabelSize {
    pub const ALL: [LabelSize; 3] = [LabelSize::Small, LabelSize::Normal, LabelSize::Large];

    pub fn px(self) -> i32 {
        match self {
            LabelSize::Small => 11,
            LabelSize::Normal => 12,
            LabelSize::Large => 14,
        }
    }

    pub fn title_px(self) -> i32 {
        match self {
            LabelSize::Small => 12,
            LabelSize::Normal => 14,
            LabelSize::Large => 17,
        }
    }
}

impl Spacing {
    pub const ALL: [Spacing; 3] = [Spacing::Compact, Spacing::Normal, Spacing::Wide];
}

pub fn cell(px: i32, labels: Option<LabelSize>, spacing: Spacing) -> i32 {
    let text = labels.map_or(24, |s| 56 + (s.px() - 12) * 4);
    let gap = match spacing {
        Spacing::Compact => -12,
        Spacing::Normal => 0,
        Spacing::Wide => 20,
    };
    (px + text + gap).max(px + 12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_sizes() {
        assert_eq!(nearest(50), 48);
        assert_eq!(nearest(0), 24);
        assert_eq!(step(48, true), 64);
        assert_eq!(step(48, false), 40);
        assert_eq!(step(128, true), 128);
        assert_eq!(step(24, false), 24);
        assert_eq!((cell(DEFAULT, Some(LabelSize::Normal), Spacing::Normal), cell(DEFAULT, None, Spacing::Normal)), (104, 72));
        assert_eq!(
            (cell(DEFAULT, Some(LabelSize::Large), Spacing::Wide), cell(DEFAULT, Some(LabelSize::Small), Spacing::Compact)),
            (132, 88)
        );
        assert_eq!((cell(24, None, Spacing::Compact), LabelSize::Small.px()), (36, 11));
    }

    #[test]
    fn sizes_have_readable_labels() {
        assert_eq!(label(48), "48 px");
        assert_eq!(LabelSize::ALL.map(LabelSize::title_px), [12, 14, 17]);
    }
}
