use super::compat;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reveal {
    #[default]
    Hover,
    Desktop,
    Never,
}

impl Reveal {
    pub const ALL: [Reveal; 3] = [Reveal::Hover, Reveal::Desktop, Reveal::Never];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Privacy {
    pub on: bool,
    pub names: bool,
    pub icons: bool,
    pub titles: bool,
    #[serde(deserialize_with = "compat::or_default")]
    pub reveal: Reveal,
}

impl Default for Privacy {
    fn default() -> Self {
        Self { on: false, names: true, icons: true, titles: false, reveal: Reveal::Hover }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mask {
    pub names: bool,
    pub icons: bool,
    pub titles: bool,
}

impl Privacy {
    pub fn mask(&self, hovered: bool, desktop: bool, peeking: bool) -> Mask {
        let shown = match self.reveal {
            Reveal::Hover => hovered || peeking,
            Reveal::Desktop => desktop || peeking,
            Reveal::Never => false,
        };
        if !self.on || shown { Mask::default() } else { Mask { names: self.names, icons: self.icons, titles: self.titles } }
    }

    pub fn hover_paints(&self) -> bool {
        self.on && self.reveal == Reveal::Hover
    }
}

impl Mask {
    pub fn any(&self) -> bool {
        self.names || self.icons || self.titles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_or_revealed_masks_nothing() {
        let p = Privacy { on: true, ..Privacy::default() };
        assert_eq!(Privacy::default().mask(false, false, false), Mask::default());
        assert_eq!(p.mask(false, false, false), Mask { names: true, icons: true, titles: false });
        assert!(!p.mask(true, false, false).any() && !p.mask(false, false, true).any());
        assert!(p.mask(false, true, false).any(), "the desktop being active does not reveal in hover mode");
        let d = Privacy { reveal: Reveal::Desktop, titles: true, ..p };
        assert!(!d.mask(false, true, false).any() && d.mask(true, false, false).titles);
        let never = Privacy { reveal: Reveal::Never, ..p };
        assert!(never.mask(true, true, true).any());
        assert!(p.hover_paints() && !d.hover_paints() && !Privacy::default().hover_paints());
    }

    #[test]
    fn unknown_reveal_values_fall_back() {
        let p: Privacy = serde_json::from_str(r#"{"on": true, "reveal": "Blink"}"#).unwrap();
        assert_eq!((p.on, p.reveal, p.names), (true, Reveal::Hover, true));
    }
}
