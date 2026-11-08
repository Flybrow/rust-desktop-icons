use super::{Config, Tab, system::Sys};
use std::collections::HashSet;

pub struct Imported {
    pub cfg: Config,
    pub orphans: Vec<Tab>,
    pub gone: Vec<Sys>,
}

impl Config {
    pub fn import(&self, other: Config) -> Imported {
        let other = Config { autostart: self.autostart, auto_update: self.auto_update, layouts: self.layouts.clone(), ..other };
        self.replace(other)
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
        Imported { cfg, orphans, gone }
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
        let i = mine.import(theirs);
        assert!(i.cfg.keep_on_desktop && !i.cfg.autostart && i.cfg.dblclick_hide);
        assert_eq!((i.cfg.desk_flags, i.cfg.fences[0].tabs[0].title.as_str()), (Some(3), "B2"));
        assert_eq!(i.orphans.iter().map(|t| t.id).collect::<Vec<_>>(), vec![1]);
        assert_eq!(i.gone, vec![Sys::RecycleBin]);
        let back = i.cfg.rewind(Config { autostart: true, keep_on_desktop: false, ..mine.clone() });
        assert!(back.cfg.autostart && back.cfg.keep_on_desktop && back.cfg.fences.len() == 2 && back.orphans.is_empty());
    }
}
