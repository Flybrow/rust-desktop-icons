use super::{Config, Sort, Tab, kind::Kind};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const AGES: [u32; 7] = [7, 14, 30, 60, 90, 180, 365];

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NameRule {
    pub pattern: String,
    pub fence: u64,
    pub tab: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Expire {
    pub days: u32,
    pub fence: u64,
    pub tab: u64,
}

pub fn glob(pattern: &str, name: &str) -> bool {
    let (p, n): (Vec<char>, Vec<char>) = (pattern.to_lowercase().chars().collect(), name.to_lowercase().chars().collect());
    let (mut pi, mut ni, mut star, mut mark) = (0, 0, None, 0);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            (pi, ni) = (pi + 1, ni + 1);
        } else if pi < p.len() && p[pi] == '*' {
            (star, mark, pi) = (Some(pi), ni, pi + 1);
        } else if let Some(s) = star {
            (pi, mark, ni) = (s + 1, mark + 1, mark + 1);
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}

pub fn expired(age_secs: u64, days: u32) -> bool {
    days > 0 && age_secs >= days as u64 * 86_400
}

impl Config {
    pub fn place_name(&self, fence: u64, tab: u64) -> String {
        self.fences.iter().find(|f| f.id == fence).map_or_else(String::new, |f| super::search::place(f, tab))
    }

    pub fn targets(&self) -> Vec<(u64, u64, String)> {
        self.fences
            .iter()
            .flat_map(|f| f.tabs.iter().filter(|t| t.portal.is_none()).map(move |t| (f.id, t.id, super::search::place(f, t.id))))
            .collect()
    }

    pub fn target_for(&self, name: &str, kinds: &[Kind]) -> Option<(u64, u64)> {
        self.name_target(name).or_else(|| self.rule_for(kinds))
    }

    pub fn move_targets(&self, here: Option<(u64, u64)>) -> Vec<(u64, Tab, String)> {
        self.fences
            .iter()
            .flat_map(|f| f.tabs.iter().map(move |t| (f.id, t.clone(), super::search::place(f, t.id))))
            .filter(|(f, t, _)| Some((*f, t.id)) != here)
            .collect()
    }

    pub fn name_target(&self, name: &str) -> Option<(u64, u64)> {
        self.name_rules.iter().find(|r| !r.pattern.trim().is_empty() && glob(r.pattern.trim(), name)).map(|r| (r.fence, r.tab))
    }

    pub fn expiring(&self) -> Vec<(u64, u64, Expire)> {
        self.fences
            .iter()
            .flat_map(|f| f.tabs.iter().filter_map(move |t| t.expire.map(|e| (f.id, t.id, e))))
            .filter(|(f, t, e)| (*f, *t) != (e.fence, e.tab) && self.tab(e.fence, e.tab).is_some())
            .collect()
    }

    pub(super) fn sanitize_rules(&mut self) {
        let exists = |c: &Config, f: u64, t: u64| c.tab(f, t).is_some_and(|t| t.portal.is_none());
        let rules = std::mem::take(&mut self.name_rules);
        self.name_rules = rules.into_iter().filter(|r| exists(self, r.fence, r.tab)).collect();
        let valid: Vec<(u64, u64)> = self.targets().into_iter().map(|(f, t, _)| (f, t)).collect();
        for t in self.fences.iter_mut().flat_map(|f| f.tabs.iter_mut()) {
            t.expire = t.expire.filter(|e| e.days > 0 && t.portal.is_none() && valid.contains(&(e.fence, e.tab)));
        }
    }
}

impl Tab {
    pub fn shows(&self, name: &str) -> bool {
        let pats: Vec<&str> = self.filter.split([';', ',']).map(str::trim).filter(|p| !p.is_empty()).collect();
        pats.is_empty() || pats.iter().any(|p| glob(p, name))
    }

    pub fn browsing(self, dir: PathBuf) -> Tab {
        let sort = if self.sort == Sort::Custom { Sort::Name } else { self.sort };
        Tab { portal: Some(dir), system: vec![], pins: vec![], order: vec![], sort, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Fence, Look, pins::Pin};

    #[test]
    fn globs() {
        assert!(glob("facture*.pdf", "Facture 2026-03.PDF"));
        assert!(glob("*.jpg", "photo.jpg") && !glob("*.jpg", "photo.jpeg"));
        assert!(glob("rapport_??.docx", "rapport_12.docx") && !glob("rapport_??.docx", "rapport_123.docx"));
        assert!(glob("*", "x") && glob("a*b*c", "a-b-b-c") && !glob("a*b*c", "a-b-b"));
        assert!(glob("été*", "Été 2026.png"));
    }

    #[test]
    fn rules_by_name_and_age() {
        let mut a = Fence::new(1, "A", (0, 0, 100, 100), Look::default());
        a.tabs.push(Tab { id: 5, expire: Some(Expire { days: 30, fence: 2, tab: 2 }), ..Tab::default() });
        let b = Fence::new(2, "B", (0, 0, 100, 100), Look::default());
        let rule = |p: &str, f, t| NameRule { pattern: p.into(), fence: f, tab: t };
        let mut c = Config {
            fences: vec![a, b],
            name_rules: vec![rule("facture*", 2, 2), rule("  ", 1, 1), rule("*.tmp", 9, 9)],
            ..Config::default()
        };
        assert_eq!((c.name_target("Facture mars.pdf"), c.name_target("photo.png")), (Some((2, 2)), None));
        c.sanitize_rules();
        assert_eq!(c.name_rules.len(), 2);
        assert_eq!((c.place_name(2, c.fences[1].tabs[0].id), c.targets().len()), ("B".into(), 3));
        assert_eq!(c.expiring(), vec![(1, 5, Expire { days: 30, fence: 2, tab: 2 })]);
        assert!(expired(31 * 86_400, 30) && !expired(29 * 86_400, 30) && !expired(u64::MAX, 0));
    }

    #[test]
    fn name_rules_win_over_kind_rules() {
        let mut a = Fence::new(1, "Docs", (0, 0, 100, 100), Look::default());
        a.tabs[0].kinds = vec![Kind::Documents];
        let mut inbox = Fence::new(2, "Inbox", (0, 0, 100, 100), Look::default());
        inbox.tabs[0].kinds = vec![Kind::Other];
        let bills = Fence::new(3, "Bills", (0, 0, 100, 100), Look::default());
        let mut c = Config { fences: vec![a, inbox, bills], ..Config::default() };
        c.name_rules = vec![NameRule { pattern: "invoice*".into(), fence: 3, tab: 3 }];
        assert_eq!(c.target_for("invoice 12.pdf", &[Kind::Documents]), Some((3, 3)));
        assert_eq!(c.target_for("report.pdf", &[Kind::Documents]), Some((1, 1)));
        assert_eq!(c.target_for("song.xyz", &[]), Some((2, 2)));
    }

    #[test]
    fn age_rules_need_a_real_source_and_target() {
        let mut a = Fence::new(1, "A", (0, 0, 100, 100), Look::default());
        a.tabs.push(Tab { id: 4, portal: Some("/p".into()), expire: Some(Expire { days: 7, fence: 2, tab: 2 }), ..Tab::default() });
        a.tabs.push(Tab { id: 5, expire: Some(Expire { days: 7, fence: 9, tab: 9 }), ..Tab::default() });
        a.tabs.push(Tab { id: 6, expire: Some(Expire { days: 0, fence: 2, tab: 2 }), ..Tab::default() });
        a.tabs.push(Tab { id: 7, expire: Some(Expire { days: 7, fence: 1, tab: 4 }), ..Tab::default() });
        a.tabs.push(Tab { id: 8, expire: Some(Expire { days: 7, fence: 2, tab: 2 }), ..Tab::default() });
        let mut c = Config { fences: vec![a, Fence::new(2, "B", (0, 0, 100, 100), Look::default())], ..Config::default() };
        c.sanitize_rules();
        let kept: Vec<u64> = c.fences[0].tabs.iter().filter(|t| t.expire.is_some()).map(|t| t.id).collect();
        assert_eq!(kept, vec![8]);
    }

    #[test]
    fn move_targets_skip_the_current_tab() {
        let mut a = Fence::new(1, "A", (0, 0, 100, 100), Look::default());
        a.tabs.push(Tab { id: 4, title: "Two".into(), ..Tab::default() });
        let c = Config { fences: vec![a, Fence::new(2, "B", (0, 0, 100, 100), Look::default())], ..Config::default() };
        let list: Vec<(u64, u64, String)> = c.move_targets(Some((1, 1))).into_iter().map(|(f, t, n)| (f, t.id, n)).collect();
        assert_eq!(list, vec![(1, 4, "A › Two".into()), (2, 2, "B".into())]);
        assert_eq!(c.move_targets(None).len(), 3);
    }

    #[test]
    fn portal_filters() {
        let t = |f: &str| Tab { filter: f.into(), ..Tab::default() };
        assert!(t("").shows("a.txt") && t(" ; ").shows("a.txt"));
        assert!(t("*.pdf").shows("Facture.PDF") && !t("*.pdf").shows("photo.jpg"));
        assert!(t("*.pdf; *.jpg").shows("photo.jpg") && t("*.pdf,rapport*").shows("rapport 2026.docx"));
    }

    #[test]
    fn a_browsed_folder_shows_like_a_portal() {
        let t = Tab { id: 3, title: "T".into(), sort: Sort::Custom, order: vec!["a".into()], pins: vec![Pin::default()], ..Tab::default() };
        let b = t.clone().browsing("/x/sub".into());
        assert_eq!((b.id, b.title.as_str(), b.portal.as_deref(), b.sort), (3, "T", Some(std::path::Path::new("/x/sub")), Sort::Name));
        assert!(b.order.is_empty() && b.pins.is_empty() && b.system.is_empty());
        assert_eq!(Tab { sort: Sort::Size, ..t }.browsing("/y".into()).sort, Sort::Size);
    }
}
