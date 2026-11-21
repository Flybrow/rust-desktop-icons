use super::Config;
use serde::{Deserialize, Serialize};

pub const DARK: u32 = 0x30_2820;
pub const LIGHT: u32 = 0xF3_F3F3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    Dark,
    Light,
    Auto,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::Dark, Theme::Light, Theme::Auto];

    pub fn light(self, system: bool) -> bool {
        match self {
            Theme::Dark => false,
            Theme::Light => true,
            Theme::Auto => system,
        }
    }
}

pub fn base(light: bool) -> u32 {
    if light { LIGHT } else { DARK }
}

impl Config {
    pub fn retheme(&mut self, light: bool) -> bool {
        if self.look.color != base(!light) {
            return false;
        }
        self.look.color = base(light);
        self.inherit();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Fence, Look, inherit::Field};

    #[test]
    fn auto_follows_windows_and_the_others_do_not() {
        assert_eq!(Theme::ALL.map(|t| t.light(true)), [false, true, true]);
        assert_eq!(Theme::ALL.map(|t| t.light(false)), [false, true, false]);
        assert_eq!(Look::default().color, DARK);
    }

    #[test]
    fn retheme_swaps_only_the_default_color() {
        let fence = |id, look| Fence::new(id, "F", (0, 0, 10, 10), look);
        let mut c = Config { fences: vec![fence(1, Look::default()), fence(2, Look::default())], ..Config::default() };
        c.inherit();
        c.fences[1].set(Field::Color, |l| l.color = 0x11_2233);
        assert!(c.retheme(true));
        assert_eq!((c.look.color, c.fences[0].look.color, c.fences[1].look.color), (LIGHT, LIGHT, 0x11_2233));
        assert!(!c.retheme(true), "already light");
        assert!(c.retheme(false));
        assert_eq!((c.look.color, c.fences[0].look.color), (DARK, DARK));
        c.look.color = 0x44_5566;
        assert!(!c.retheme(true) && c.look.color == 0x44_5566, "a color picked by the user is kept");
    }
}
