use super::{
    compat,
    grid::Align,
    icons::{self, LabelSize, Spacing},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Look {
    pub color: u32,
    pub alpha: u8,
    pub icon: i32,
    pub tint: Option<u32>,
    pub chameleon: bool,
    pub auto_height: bool,
    pub labels: bool,
    #[serde(deserialize_with = "compat::or_default")]
    pub align: Align,
    #[serde(deserialize_with = "compat::or_default")]
    pub label_size: LabelSize,
    pub label_color: Option<u32>,
    #[serde(deserialize_with = "compat::or_default")]
    pub spacing: Spacing,
    pub browse: bool,
    #[serde(deserialize_with = "compat::or_default")]
    pub title_align: Align,
    #[serde(deserialize_with = "compat::or_default")]
    pub title_size: LabelSize,
    pub bar_color: Option<u32>,
    pub bar_hover: bool,
    pub wallpaper: bool,
    pub thumbs: bool,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            color: 0x302820,
            alpha: 217,
            icon: icons::DEFAULT,
            tint: None,
            chameleon: false,
            auto_height: true,
            labels: true,
            align: Align::Left,
            label_size: LabelSize::Normal,
            label_color: None,
            spacing: Spacing::Normal,
            browse: true,
            title_align: Align::Center,
            title_size: LabelSize::Normal,
            bar_color: None,
            bar_hover: false,
            wallpaper: false,
            thumbs: true,
        }
    }
}

impl Look {
    pub const PERCENTS: [u32; 8] = [30, 40, 50, 60, 70, 80, 90, 100];

    pub fn percent(&self) -> u32 {
        (self.alpha as u32 * 100 + 127) / 255
    }

    pub fn label(pct: u32) -> String {
        format!("{pct}%")
    }

    pub fn alpha_for(pct: u32) -> u8 {
        (pct.min(100) * 255 / 100) as u8
    }

    pub(super) fn sanitized(mut self) -> Self {
        self.alpha = self.alpha.max(64);
        self.color &= 0xFF_FFFF;
        self.icon = icons::nearest(self.icon);
        self.tint = self.tint.map(|t| t & 0xFF_FFFF);
        self.label_color = self.label_color.map(|t| t & 0xFF_FFFF);
        self.bar_color = self.bar_color.map(|t| t & 0xFF_FFFF);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_steps() {
        assert_eq!((Look::alpha_for(30), Look::alpha_for(100), Look::alpha_for(500)), (76, 255, 255));
        assert_eq!(Look { alpha: Look::alpha_for(70), ..Look::default() }.percent(), 70);
    }
}
