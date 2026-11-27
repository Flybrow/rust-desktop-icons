mod follow;
mod slots;

use super::{Config, Tab};
pub use follow::{GRACE_MS, Listing, Missing};
use serde::{Deserialize, Serialize};
pub use slots::{Wall, clear_spot, corner_slot, covered, free_slots};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pin {
    pub path: PathBuf,
    pub id: u64,
}

pub fn key(p: &Path) -> String {
    p.to_string_lossy().to_lowercase()
}

pub fn keys<'a>(paths: impl IntoIterator<Item = &'a PathBuf>) -> HashSet<String> {
    paths.into_iter().map(|p| key(p)).collect()
}

impl Tab {
    pub fn pinned(&self, p: &Path) -> bool {
        let k = key(p);
        self.pins.iter().any(|x| key(&x.path) == k)
    }
}

impl Config {
    pub fn pinned(&self) -> Vec<PathBuf> {
        self.fences.iter().flat_map(|f| &f.tabs).flat_map(|t| &t.pins).map(|p| p.path.clone()).collect()
    }

    pub fn pin_keys(&self) -> HashSet<String> {
        self.fences.iter().flat_map(|f| &f.tabs).flat_map(|t| &t.pins).map(|p| key(&p.path)).collect()
    }

    pub fn is_pinned(&self, p: &Path) -> bool {
        self.fences.iter().flat_map(|f| &f.tabs).any(|t| t.pinned(p))
    }

    pub fn unpin(&mut self, paths: &[PathBuf]) {
        let gone = keys(paths);
        for t in self.fences.iter_mut().flat_map(|f| f.tabs.iter_mut()) {
            t.pins.retain(|p| !gone.contains(&key(&p.path)));
        }
    }

    pub fn pin(&mut self, fence: u64, tab: u64, pins: Vec<Pin>) {
        let paths: Vec<PathBuf> = pins.iter().map(|p| p.path.clone()).collect();
        self.unpin(&paths);
        if let Some(t) = self.tab_mut(fence, tab) {
            t.pins.extend(pins);
        }
    }
}

#[cfg(test)]
fn pin(p: &str, id: u64) -> Pin {
    Pin { path: p.into(), id }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Fence, Look};

    #[test]
    fn pins_move_between_tabs() {
        let mut a = Fence::new(1, "A", (0, 0, 200, 200), Look::default());
        a.tabs.push(Tab { id: 5, ..Tab::default() });
        let mut c = Config { fences: vec![a], ..Config::default() };
        c.pin(1, 1, vec![pin("D:\\a.txt", 1), pin("D:\\b.txt", 2)]);
        c.pin(1, 5, vec![pin("d:\\A.txt", 1)]);
        assert_eq!(c.tab(1, 1).unwrap().pins, vec![pin("D:\\b.txt", 2)]);
        assert!(c.tab(1, 5).unwrap().pinned(Path::new("D:\\a.txt")) && c.is_pinned(Path::new("D:\\B.TXT")));
        c.unpin(&["D:\\b.txt".into()]);
        assert_eq!(c.pinned(), vec![PathBuf::from("d:\\A.txt")]);
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<Config>(&json).unwrap().sanitized().pinned(), c.pinned());
    }

    #[test]
    fn pins_are_found_only_where_they_are() {
        let mut c = Config { fences: vec![Fence::new(1, "A", (0, 0, 9, 9), Look::default())], ..Config::default() };
        c.fences[0].tabs[0].pins = vec![pin("D:\\a.txt", 1), pin("D:\\Été.txt", 2)];
        assert!(c.is_pinned(Path::new("d:\\A.TXT")) && !c.is_pinned(Path::new("D:\\b.txt")));
        assert!(c.is_pinned(Path::new("d:\\éTÉ.txt")) && !Tab::default().pinned(Path::new("D:\\a.txt")));
        let set = c.pin_keys();
        assert!(set.contains(&key(Path::new("D:\\A.TXT"))) && set.contains("d:\\été.txt") && set.len() == 2);
        assert_eq!(keys(&[PathBuf::from("X:\\A"), PathBuf::from("x:\\a")]).len(), 1);
    }
}
