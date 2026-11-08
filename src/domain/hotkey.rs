use super::Config;
use serde::{Deserialize, Serialize};

pub const ALT: u32 = 1;
pub const CTRL: u32 = 2;
pub const SHIFT: u32 = 4;
pub const WIN: u32 = 8;

const RESERVED: [(u32, u32); 11] = [
    (ALT, 0x73),
    (ALT, 0x20),
    (ALT, 0x09),
    (CTRL, 0x1B),
    (CTRL | ALT, 0x2E),
    (CTRL, 0x41),
    (CTRL, 0x43),
    (CTRL, 0x53),
    (CTRL, 0x56),
    (CTRL, 0x58),
    (CTRL, 0x5A),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Peek,
    Search,
    HideAll,
    NewFence,
    Settings,
}

impl Slot {
    pub const ALL: [Slot; 5] = [Slot::Peek, Slot::Search, Slot::HideAll, Slot::NewFence, Slot::Settings];

    pub fn id(self) -> i32 {
        self as i32 + 1
    }

    pub fn default_key(self) -> Option<Hotkey> {
        match self {
            Slot::Peek => Some(Hotkey::PEEK),
            Slot::Search => Some(Hotkey::SEARCH),
            _ => None,
        }
    }

    pub fn of(id: i32) -> Option<Slot> {
        Slot::ALL.into_iter().find(|s| s.id() == id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hotkey {
    pub mods: u32,
    pub vk: u32,
}

fn modifier(vk: u32) -> bool {
    matches!(vk, 0x10..=0x12 | 0x5B | 0x5C | 0xA0..=0xA5)
}

fn key_name(vk: u32) -> String {
    match vk {
        0x30..=0x39 | 0x41..=0x5A => char::from(vk as u8).to_string(),
        0x60..=0x69 => format!("Num {}", vk - 0x60),
        0x70..=0x87 => format!("F{}", vk - 0x6F),
        0x20 => "Space".into(),
        0x0D => "Enter".into(),
        0x09 => "Tab".into(),
        0x2D => "Insert".into(),
        0x2E => "Delete".into(),
        0x24 => "Home".into(),
        0x23 => "End".into(),
        0x21 => "PageUp".into(),
        0x22 => "PageDown".into(),
        0x25 => "←".into(),
        0x26 => "↑".into(),
        0x27 => "→".into(),
        0x28 => "↓".into(),
        _ => format!("#{vk:02X}"),
    }
}

impl Hotkey {
    pub const PEEK: Hotkey = Hotkey { mods: CTRL | ALT, vk: 0x20 };
    pub const SEARCH: Hotkey = Hotkey { mods: CTRL | ALT, vk: 0x46 };

    pub fn valid(&self) -> bool {
        self.mods & (CTRL | ALT | WIN) != 0 && !modifier(self.vk) && self.vk != 0 && !RESERVED.contains(&(self.mods, self.vk))
    }

    pub fn label(&self) -> String {
        let names = [(CTRL, "Ctrl"), (ALT, "Alt"), (SHIFT, "Shift"), (WIN, "Win")];
        let mut parts: Vec<String> = names.iter().filter(|(m, _)| self.mods & m != 0).map(|(_, n)| n.to_string()).collect();
        parts.push(key_name(self.vk));
        parts.join("+")
    }
}

impl Config {
    fn slot(&mut self, s: Slot) -> &mut Option<Hotkey> {
        match s {
            Slot::Peek => &mut self.peek_key,
            Slot::Search => &mut self.search_key,
            Slot::HideAll => &mut self.hide_key,
            Slot::NewFence => &mut self.new_key,
            Slot::Settings => &mut self.settings_key,
        }
    }

    pub fn key(&self, s: Slot) -> Option<Hotkey> {
        match s {
            Slot::Peek => self.peek_key,
            Slot::Search => self.search_key,
            Slot::HideAll => self.hide_key,
            Slot::NewFence => self.new_key,
            Slot::Settings => self.settings_key,
        }
    }

    pub fn set_key(&mut self, s: Slot, k: Option<Hotkey>) -> bool {
        if k.is_some() && Slot::ALL.iter().any(|o| *o != s && self.key(*o) == k) {
            return false;
        }
        *self.slot(s) = k.filter(Hotkey::valid);
        true
    }

    pub(super) fn sanitize_keys(&mut self) {
        let mut used = vec![];
        for s in Slot::ALL {
            let k = self.key(s).filter(|k| k.valid() && !used.contains(k));
            used.extend(k);
            *self.slot(s) = k;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_validity() {
        assert_eq!(Hotkey::PEEK.label(), "Ctrl+Alt+Space");
        assert_eq!(Hotkey { mods: WIN | SHIFT, vk: 0x71 }.label(), "Shift+Win+F2");
        assert_eq!(Hotkey { mods: CTRL, vk: 0xBA }.label(), "Ctrl+#BA");
        assert!(Hotkey::PEEK.valid() && Hotkey::SEARCH.valid());
        assert!(!Hotkey { mods: SHIFT, vk: 0x41 }.valid());
        assert!(!Hotkey { mods: CTRL, vk: 0x11 }.valid());
        assert!(!Hotkey { mods: ALT, vk: 0x73 }.valid() && !Hotkey { mods: CTRL, vk: 0x43 }.valid());
        assert!(Hotkey { mods: CTRL | SHIFT, vk: 0x43 }.valid());
        let back: Hotkey = serde_json::from_str(&serde_json::to_string(&Hotkey::PEEK).unwrap()).unwrap();
        assert_eq!(back, Hotkey::PEEK);
    }

    #[test]
    fn slots_never_share_a_shortcut() {
        let mut c = Config::default();
        assert_eq!((Slot::of(1), Slot::of(2), Slot::of(9)), (Some(Slot::Peek), Some(Slot::Search), None));
        assert!(Slot::ALL.iter().all(|s| c.key(*s) == s.default_key()));
        assert!(c.set_key(Slot::HideAll, Some(Hotkey { mods: CTRL | ALT, vk: 0x48 })) && c.key(Slot::HideAll).is_some());
        assert!(!c.set_key(Slot::Search, Some(Hotkey::PEEK)) && c.key(Slot::Search) == Some(Hotkey::SEARCH));
        assert!(c.set_key(Slot::Search, None) && c.key(Slot::Search).is_none());
        assert!(c.set_key(Slot::Peek, Some(Hotkey::SEARCH)));
        c.search_key = Some(Hotkey::SEARCH);
        c.peek_key = Some(Hotkey { mods: 0, vk: 0x20 });
        c.sanitize_keys();
        assert_eq!((c.peek_key, c.search_key), (None, Some(Hotkey::SEARCH)));
        c.peek_key = Some(Hotkey::SEARCH);
        c.sanitize_keys();
        assert_eq!((c.peek_key, c.search_key), (Some(Hotkey::SEARCH), None));
    }
}
