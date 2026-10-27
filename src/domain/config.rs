use super::{
    anim,
    kind::Kind,
    model::{Fence, Look, Tab},
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub desk_icons: Vec<(Sys, Option<u32>)>,
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
        }
    }
}

impl Config {
    pub fn next_id(&self) -> u64 {
        self.fences.iter().flat_map(|f| f.tabs.iter().map(|t| t.id).chain([f.id])).max().map_or(1, |m| m + 1)
    }

    pub fn placed(&self) -> Vec<Sys> {
        Sys::ALL.into_iter().filter(|s| self.fences.iter().flat_map(|f| &f.tabs).any(|t| t.system.contains(s))).collect()
    }

    pub fn remember(&mut self, s: Sys, prev: Option<u32>) -> bool {
        let fresh = !self.desk_icons.iter().any(|(x, _)| *x == s);
        if fresh {
            self.desk_icons.push((s, prev));
        }
        fresh
    }

    pub fn forget(&mut self, s: Sys) -> Option<Option<u32>> {
        self.desk_icons.iter().position(|(x, _)| *x == s).map(|i| self.desk_icons.remove(i).1)
    }

    pub fn original(&self, s: Sys) -> Option<u32> {
        self.desk_icons.iter().find(|(x, _)| *x == s).and_then(|(_, p)| *p)
    }

    pub fn place(&mut self, s: Sys, tab: Option<u64>) {
        for t in self.fences.iter_mut().flat_map(|f| f.tabs.iter_mut()) {
            t.system.retain(|x| *x != s);
            if Some(t.id) == tab {
                t.system.push(s);
            }
        }
    }

    pub fn tab(&self, fence: u64, tab: u64) -> Option<&Tab> {
        self.fences.iter().find(|f| f.id == fence)?.tabs.iter().find(|t| t.id == tab)
    }

    pub fn tab_mut(&mut self, fence: u64, tab: u64) -> Option<&mut Tab> {
        self.fences.iter_mut().find(|f| f.id == fence)?.tabs.iter_mut().find(|t| t.id == tab)
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

    pub fn sanitized(mut self) -> Self {
        let mut seen = HashSet::new();
        self.fences = self.fences.into_iter().filter(|f| f.id > 0 && seen.insert(f.id)).map(Fence::sanitized).collect();
        let mut tabs = HashSet::new();
        for f in &mut self.fences {
            f.tabs.retain(|t| t.id > 0 && tabs.insert(t.id));
            if f.tabs.is_empty() {
                f.tabs.push(Tab { id: f.id, ..Tab::default() });
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
        assert!(c.auto_update && c.autostart && c.auto_sort && f.look.auto_height && !c.keep_on_desktop);
        assert_eq!((f.w, f.look.color, f.look.icon, f.tabs.len(), f.active().title.as_str()), (360, 255, 48, 1, "Jeux"));
        assert_eq!(c.roll_ms, 500);
    }

    #[test]
    fn system_icons_live_in_one_tab() {
        let c: Config = serde_json::from_str::<Config>(
            r#"{"fences":[{"id":1,"tabs":[{"id":1},{"id":5,"recycle":true}]},{"id":2,"tabs":[{"id":2,"recycle":true,"system":["ThisPc"]}]}]}"#,
        )
        .unwrap();
        let mut c = c.sanitized();
        let holders =
            |c: &Config, s: Sys| c.fences.iter().flat_map(|f| &f.tabs).filter(|t| t.system.contains(&s)).map(|t| t.id).collect::<Vec<_>>();
        assert_eq!((holders(&c, Sys::RecycleBin), holders(&c, Sys::ThisPc)), (vec![5], vec![2]));
        assert!(!serde_json::to_string(&c).unwrap().contains("recycle"));
        c.place(Sys::RecycleBin, Some(2));
        assert_eq!(c.tab(2, 2).unwrap().system, vec![Sys::ThisPc, Sys::RecycleBin]);
        assert!(c.tab(1, 5).unwrap().system.is_empty());
        c.place(Sys::ThisPc, None);
        assert_eq!(c.placed(), vec![Sys::RecycleBin]);
    }

    #[test]
    fn desktop_visibility_is_remembered_once() {
        let mut c = Config::default();
        assert!(c.remember(Sys::ThisPc, Some(0)));
        assert!(!c.remember(Sys::ThisPc, Some(1)));
        assert!(c.remember(Sys::Network, None));
        assert_eq!((c.original(Sys::ThisPc), c.original(Sys::Network), c.original(Sys::UserFiles)), (Some(0), None, None));
        let back: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back.desk_icons, c.desk_icons);
        assert_eq!(c.forget(Sys::ThisPc), Some(Some(0)));
        assert_eq!((c.forget(Sys::ThisPc), c.forget(Sys::Network)), (None, Some(None)));
        assert!(c.desk_icons.is_empty());
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
        c.auto_sort = false;
        c.toggle_rule(1, 5, Kind::Music);
        assert!(c.auto_sort && c.tab(1, 5).unwrap().kinds.contains(&Kind::Music));
        c.auto_sort = false;
        c.toggle_rule(1, 5, Kind::Music);
        assert!(!c.auto_sort && !c.tab(1, 5).unwrap().kinds.contains(&Kind::Music));
        assert_eq!(c.next_id(), 7);
    }
}
