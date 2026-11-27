use super::{Config, Tab, ids::Ids, system::Sys};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

const MACHINE: &str = "machine";

pub struct Imported {
    pub cfg: Config,
    pub orphans: Vec<Tab>,
    pub gone: Vec<Sys>,
    pub foreign: bool,
}

pub fn machine_id(guid: &str) -> String {
    Sha256::digest(format!("rust-desktop-icons/{}", guid.trim().to_lowercase())).iter().map(|b| format!("{b:02x}")).collect()
}

pub fn export(c: &Config, machine: Option<&str>) -> serde_json::Result<Vec<u8>> {
    let json = serde_json::to_vec_pretty(c)?;
    Ok(match machine {
        Some(m) if json.starts_with(b"{") => {
            [format!("{{\n  \"{MACHINE}\": {},", serde_json::to_string(m)?).as_bytes(), &json[1..]].concat()
        }
        _ => json,
    })
}

pub fn machine_of(bytes: &[u8]) -> Option<String> {
    #[derive(Deserialize)]
    struct Stamp {
        machine: Option<String>,
    }
    serde_json::from_slice::<Stamp>(super::config::unbom(bytes)).ok()?.machine
}

pub fn foreign(from: Option<&str>, here: Option<&str>) -> bool {
    from.is_some_and(|f| here != Some(f))
}

impl Config {
    pub fn import(&self, other: Config, foreign: bool, dirs: &[u64]) -> Imported {
        let other =
            Config { autostart: self.autostart, auto_update: self.auto_update, theme: self.theme, layouts: self.layouts.clone(), ..other };
        let other = if foreign { other.sanitized().renumbered(self.ids(dirs)) } else { other };
        let mut i = self.replace(other);
        if foreign {
            (i.orphans, i.foreign) = (vec![], true);
        }
        i
    }

    fn renumbered(mut self, mut ids: Ids) -> Config {
        let mut map = HashMap::new();
        let mut new = |old: u64| *map.entry(old).or_insert_with(|| ids.take());
        for f in &mut self.fences {
            f.id = new(f.id);
            for t in &mut f.tabs {
                t.id = new(t.id);
                if let Some(e) = &mut t.expire {
                    (e.fence, e.tab) = (new(e.fence), new(e.tab));
                }
            }
        }
        for r in &mut self.name_rules {
            (r.fence, r.tab) = (new(r.fence), new(r.tab));
        }
        for (id, _) in self.snapshots.iter_mut().flat_map(|s| &mut s.layout.fences) {
            *id = new(*id);
        }
        self
    }

    pub fn rewind(&self, before: Config) -> Imported {
        self.replace(before)
    }

    fn replace(&self, other: Config) -> Imported {
        let cfg = Config {
            version: self.version.clone(),
            keep_on_desktop: self.keep_on_desktop,
            desk_flags: self.desk_flags,
            desk_icons: self.desk_icons.clone(),
            screens: self.screens.clone(),
            ..other
        }
        .sanitized();
        let kept: HashSet<u64> = cfg.fences.iter().flat_map(|f| &f.tabs).filter(|t| t.portal.is_none()).map(|t| t.id).collect();
        let orphans = self.fences.iter().flat_map(|f| &f.tabs).filter(|t| t.portal.is_none() && !kept.contains(&t.id)).cloned().collect();
        let placed = cfg.placed();
        let gone = self.placed().into_iter().filter(|s| !placed.contains(s)).collect();
        Imported { cfg, orphans, gone, foreign: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Fence, Look};

    #[test]
    fn import_keeps_machine_state() {
        let mut mine = Config { keep_on_desktop: true, autostart: false, desk_flags: Some(3), ..Config::default() };
        mine.fences = vec![Fence::new(1, "A", (0, 0, 9, 9), Look::default()), Fence::new(2, "B", (0, 0, 9, 9), Look::default())];
        mine.fences[1].tabs[0].system = vec![Sys::RecycleBin];
        mine.fences[1].tabs.push(Tab { id: 3, portal: Some("/x".into()), ..Tab::default() });
        let theirs = Config { fences: vec![Fence::new(2, "B2", (5, 5, 9, 9), Look::default())], dblclick_hide: true, ..Config::default() };
        let i = mine.import(theirs, false, &[]);
        assert!(!i.foreign && i.cfg.keep_on_desktop && !i.cfg.autostart && i.cfg.dblclick_hide);
        assert_eq!((i.cfg.desk_flags, i.cfg.fences[0].tabs[0].title.as_str()), (Some(3), "B2"));
        assert_eq!(i.orphans.iter().map(|t| t.id).collect::<Vec<_>>(), vec![1]);
        assert_eq!(i.gone, vec![Sys::RecycleBin]);
        let back = i.cfg.rewind(Config { autostart: true, keep_on_desktop: false, ..mine.clone() });
        assert!(back.cfg.autostart && back.cfg.keep_on_desktop && back.cfg.fences.len() == 2 && back.orphans.is_empty());
    }

    #[test]
    fn an_export_from_another_pc_gets_fresh_ids_and_keeps_local_folders() {
        use crate::domain::{matching::Expire, matching::NameRule};
        let mine = Config { fences: vec![Fence::new(1, "A", (0, 0, 9, 9), Look::default())], ..Config::default() };
        let mut theirs = Config { fences: vec![Fence::new(1, "X", (0, 0, 9, 9), Look::default())], ..Config::default() };
        theirs.fences[0].tabs.push(Tab { id: 2, expire: Some(Expire { days: 3, fence: 1, tab: 1 }), ..Tab::default() });
        theirs.name_rules = vec![NameRule { pattern: "*.pdf".into(), fence: 1, tab: 2 }];
        theirs.snapshot("S".into());
        let bytes = export(&theirs, Some(&machine_id("{ABC}"))).unwrap();
        assert!(bytes.starts_with(b"{\n  \"machine\": \"") && serde_json::from_slice::<serde_json::Value>(&bytes).is_ok());
        let (from, parsed) = (machine_of(&bytes), Config::parse(&bytes).unwrap());
        assert_eq!(from.as_deref(), Some(machine_id(" {abc} ").as_str()));
        assert!(foreign(from.as_deref(), Some(&machine_id("{other}"))) && foreign(from.as_deref(), None));
        assert!(!foreign(from.as_deref(), Some(&machine_id("{ABC}"))) && !foreign(None, Some("x")));
        assert_eq!(machine_of(&export(&theirs, None).unwrap()), None);
        let i = mine.import(parsed, true, &[5, 6]);
        assert!(i.foreign && i.orphans.is_empty());
        let f = &i.cfg.fences[0];
        assert_eq!((f.id, f.tabs[0].id, f.tabs[1].id, f.tabs[0].title.as_str()), (7, 7, 8, "X"));
        assert_eq!(f.tabs[1].expire, Some(Expire { days: 3, fence: 7, tab: 7 }));
        assert_eq!((i.cfg.name_rules[0].fence, i.cfg.name_rules[0].tab), (7, 8));
        assert_eq!(i.cfg.snapshots[0].layout.fences[0].0, 7);
        let same = mine.import(Config::parse(&bytes).unwrap(), false, &[5, 6]);
        assert!(!same.foreign && same.cfg.fences[0].id == 1 && same.orphans.is_empty());
    }

    #[test]
    fn this_pc_keeps_its_own_screens_and_updates() {
        let screens = vec![crate::domain::screens::Screen { full: [0, 0, 1920, 1080], work: [0, 0, 1920, 1040] }];
        let mine = Config {
            auto_update: false,
            version: Some("1.0.0".into()),
            desk_icons: vec![(Sys::ThisPc, Some(1))],
            screens: screens.clone(),
            layouts: vec![Default::default()],
            ..Config::default()
        };
        let theirs = Config { auto_update: true, version: Some("0.9".into()), layouts: vec![], ..Config::default() };
        let i = mine.import(theirs.clone(), false, &[]);
        assert_eq!((i.cfg.auto_update, i.cfg.version.as_deref(), i.cfg.layouts.len()), (false, Some("1.0.0"), 1));
        assert_eq!((i.cfg.desk_icons.clone(), i.cfg.screens.clone()), (vec![(Sys::ThisPc, Some(1))], screens.clone()));
        let back = mine.rewind(theirs);
        assert_eq!((back.cfg.auto_update, back.cfg.screens, back.cfg.version.as_deref()), (true, screens, Some("1.0.0")));
    }
}
