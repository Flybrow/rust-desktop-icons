use super::{
    anim, compat,
    hotkey::Hotkey,
    kind::Kind,
    layouts::{Layout, Snapshot},
    look::Look,
    matching::NameRule,
    model::{Fence, Tab},
    screens::{Adapt, Screen},
    system::Sys,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub lang: Option<String>,
    pub auto_update: bool,
    pub autostart: bool,
    pub auto_sort: bool,
    pub roll_ms: u32,
    pub look: Look,
    pub fences: Vec<Fence>,
    pub version: Option<String>,
    pub keep_on_desktop: bool,
    pub desk_flags: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty", deserialize_with = "compat::known")]
    pub desk_icons: Vec<(Sys, Option<u32>)>,
    #[serde(deserialize_with = "compat::or_default")]
    pub screen_adapt: Adapt,
    pub dblclick_hide: bool,
    pub peek_key: Option<Hotkey>,
    pub search_key: Option<Hotkey>,
    pub hide_key: Option<Hotkey>,
    pub new_key: Option<Hotkey>,
    pub settings_key: Option<Hotkey>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub name_rules: Vec<NameRule>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub screens: Vec<Screen>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub layouts: Vec<Layout>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub snapshots: Vec<Snapshot>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lang: None,
            auto_update: true,
            autostart: true,
            auto_sort: true,
            roll_ms: anim::DEFAULT_MS,
            look: Look::default(),
            fences: vec![],
            version: None,
            keep_on_desktop: false,
            desk_flags: None,
            desk_icons: vec![],
            screen_adapt: Adapt::default(),
            dblclick_hide: false,
            peek_key: Some(Hotkey::PEEK),
            search_key: Some(Hotkey::SEARCH),
            hide_key: None,
            new_key: None,
            settings_key: None,
            name_rules: vec![],
            screens: vec![],
            layouts: vec![],
            snapshots: vec![],
        }
    }
}

impl Config {
    pub fn next_id(&self) -> u64 {
        self.fences.iter().flat_map(|f| f.tabs.iter().map(|t| t.id).chain([f.id])).max().map_or(1, |m| m + 1)
    }

    pub fn fence(&self, id: u64) -> Option<&Fence> {
        self.fences.iter().find(|f| f.id == id)
    }

    pub fn fence_mut(&mut self, id: u64) -> Option<&mut Fence> {
        self.fences.iter_mut().find(|f| f.id == id)
    }

    pub fn tab(&self, fence: u64, tab: u64) -> Option<&Tab> {
        self.fence(fence)?.tabs.iter().find(|t| t.id == tab)
    }

    pub fn tab_mut(&mut self, fence: u64, tab: u64) -> Option<&mut Tab> {
        self.fence_mut(fence)?.tabs.iter_mut().find(|t| t.id == tab)
    }

    pub fn toggle_rule(&mut self, fence: u64, tab: u64, kind: Kind) {
        let Some(t) = self.tab_mut(fence, tab) else { return };
        if t.kinds.contains(&kind) {
            t.kinds.retain(|x| *x != kind);
        } else {
            t.kinds.push(kind);
            self.auto_sort = true;
        }
    }

    pub fn rule_target(&self, kind: Kind) -> Option<(u64, u64)> {
        self.fences.iter().find_map(|f| f.tabs.iter().find(|t| t.portal.is_none() && t.kinds.contains(&kind)).map(|t| (f.id, t.id)))
    }

    pub fn rule_for(&self, kinds: &[Kind]) -> Option<(u64, u64)> {
        kinds.iter().chain([&Kind::Other]).find_map(|k| self.rule_target(*k))
    }

    pub fn parse(bytes: &[u8]) -> serde_json::Result<Self> {
        let json = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
        if json.trim_ascii_start().first() != Some(&b'{') {
            return Err(serde::de::Error::custom("not a JSON object"));
        }
        serde_json::from_slice(json)
    }

    pub fn sanitized(mut self) -> Self {
        self.sanitize_keys();
        self.sanitize_rules();
        let mut seen = HashSet::new();
        self.fences = self.fences.into_iter().filter(|f| f.id > 0 && seen.insert(f.id)).map(Fence::sanitized).collect();
        let mut tabs = HashSet::new();
        let mut fresh = self.next_id();
        for f in &mut self.fences {
            f.tabs.retain(|t| t.id > 0 && tabs.insert(t.id));
            if f.tabs.is_empty() {
                let id = if tabs.insert(f.id) {
                    f.id
                } else {
                    fresh += 1;
                    fresh - 1
                };
                tabs.insert(id);
                f.tabs.push(Tab { id, ..Tab::default() });
            }
            f.tab = f.tab.min(f.tabs.len() - 1);
        }
        let mut seen = HashSet::new();
        for t in self.fences.iter_mut().flat_map(|f| f.tabs.iter_mut()) {
            if std::mem::take(&mut t.recycle) {
                t.system.insert(0, Sys::RecycleBin);
            }
            t.system.retain(|s| seen.insert(*s));
        }
        self.look = self.look.sanitized();
        self.inherit();
        self.roll_ms = *anim::SPEEDS.iter().min_by_key(|s| s.abs_diff(self.roll_ms)).unwrap_or(&anim::DEFAULT_MS);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrip_and_sanitize() {
        let look = Look { alpha: 0, color: 0xFF12_3456, ..Look::default() };
        let c = Config {
            fences: vec![
                Fence { id: 3, w: 5, look, title: "a\nb".into(), ..Fence::default() },
                Fence { id: 3, ..Fence::default() },
                Fence { id: 0, ..Fence::default() },
            ],
            ..Config::default()
        };
        let c: Config = serde_json::from_str::<Config>(&serde_json::to_string(&c).unwrap()).unwrap().sanitized();
        assert_eq!(c.fences.len(), 1);
        let f = &c.fences[0];
        assert_eq!((f.w, f.look.alpha, f.look.color, f.active().title.as_str()), (Fence::MIN.0, 64, 0x12_3456, "ab"));
        assert_eq!((f.active().id, f.title.as_str()), (3, ""));
        assert_eq!(c.next_id(), 4);
        assert_eq!(Config::default().next_id(), 1);
    }

    #[test]
    fn legacy_json_migrates() {
        let c: Config = serde_json::from_str::<Config>(r#"{"fences":[{"id":1,"title":"Jeux","color":255,"icon":50}],"roll_ms":999}"#)
            .unwrap()
            .sanitized();
        let f = &c.fences[0];
        assert!(c.auto_update && c.autostart && c.auto_sort && f.look.auto_height && f.look.labels && !c.keep_on_desktop);
        assert!(!c.dblclick_hide && c.peek_key == Some(Hotkey::PEEK));
        let set: Config = serde_json::from_str(r#"{"peek_key":null,"dblclick_hide":true}"#).unwrap();
        assert!(set.dblclick_hide && set.peek_key.is_none());
        let bom = Config::parse("\u{feff}{\"dblclick_hide\":true}".as_bytes()).unwrap();
        assert!(bom.dblclick_hide && Config::parse(b"{\"dblclick_hide\":true}").unwrap().dblclick_hide);
        assert_eq!((f.w, f.look.color, f.look.icon, f.tabs.len(), f.active().title.as_str()), (360, 255, 48, 1, "Jeux"));
        assert_eq!(c.roll_ms, 500);
    }

    #[test]
    fn rules_pick_first_matching_tab() {
        let mut a = Fence::new(1, "A", (0, 0, 200, 200), Look::default());
        a.tabs.push(Tab { id: 5, kinds: vec![Kind::Images], ..Tab::default() });
        let mut b = Fence::new(2, "B", (0, 0, 200, 200), Look::default());
        b.tabs[0].kinds = vec![Kind::Images, Kind::Apps];
        b.tabs.push(Tab { id: 6, portal: Some("C:\\x".into()), kinds: vec![Kind::Music], ..Tab::default() });
        let mut c = Config { fences: vec![a, b], ..Config::default() };
        assert_eq!(c.rule_target(Kind::Images), Some((1, 5)));
        assert_eq!(c.rule_target(Kind::Apps), Some((2, 2)));
        assert_eq!(c.rule_target(Kind::Music), None);
        assert_eq!((c.rule_for(&[Kind::Music]), c.rule_for(&[])), (None, None));
        c.toggle_rule(2, 2, Kind::Other);
        assert_eq!((c.rule_for(&[Kind::Music]), c.rule_for(&[Kind::Images])), (Some((2, 2)), Some((1, 5))));
        c.toggle_rule(2, 2, Kind::Other);
        c.auto_sort = false;
        c.toggle_rule(1, 5, Kind::Music);
        assert!(c.auto_sort && c.tab(1, 5).unwrap().kinds.contains(&Kind::Music));
        c.auto_sort = false;
        c.toggle_rule(1, 5, Kind::Music);
        assert!(!c.auto_sort && !c.tab(1, 5).unwrap().kinds.contains(&Kind::Music));
        assert_eq!(c.next_id(), 7);
    }
}
