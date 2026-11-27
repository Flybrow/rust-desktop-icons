use super::{Config, TABS, Tab};
use std::collections::HashSet;

pub struct Ids {
    used: HashSet<u64>,
    next: u64,
}

impl Ids {
    pub fn take(&mut self) -> u64 {
        loop {
            let id = self.next;
            self.next = self.next.checked_add(1).unwrap_or(1);
            if id > 0 && self.used.insert(id) {
                return id;
            }
        }
    }
}

impl Config {
    pub fn ids(&self, extra: &[u64]) -> Ids {
        let used: HashSet<u64> =
            self.fences.iter().flat_map(|f| f.tabs.iter().map(|t| t.id).chain([f.id])).chain(extra.iter().copied()).collect();
        let next = used.iter().max().map_or(1, |m| m.checked_add(1).unwrap_or(1));
        Ids { used, next }
    }

    pub fn next_id(&self) -> u64 {
        self.ids(&[]).take()
    }

    pub(super) fn sanitize_ids(&mut self) {
        let mut kept = HashSet::new();
        for f in &mut self.fences {
            f.tabs.retain(|t| t.id > 0 && kept.insert(t.id));
            f.tabs.truncate(TABS);
        }
        let mut ids = self.ids(&kept.iter().copied().collect::<Vec<_>>());
        for f in &mut self.fences {
            if f.tabs.is_empty() {
                let id = if kept.insert(f.id) { f.id } else { ids.take() };
                f.tabs.push(Tab { id, ..Tab::default() });
            }
            f.tab = f.tab.min(f.tabs.len() - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Fence, Look};

    fn fence(id: u64, tabs: &[(u64, &str)]) -> Fence {
        let tabs = tabs.iter().map(|&(id, title)| Tab { id, title: title.into(), ..Tab::default() }).collect();
        Fence { id, tabs, ..Fence::default() }
    }

    fn tabs(c: &Config) -> Vec<(u64, Vec<u64>)> {
        c.fences.iter().map(|f| (f.id, f.tabs.iter().map(|t| t.id).collect())).collect()
    }

    #[test]
    fn an_emptied_fence_never_takes_a_later_fence_tab() {
        let c = Config { fences: vec![fence(1, &[(3, "")]), fence(2, &[(3, "")]), fence(4, &[(2, "Work")])], ..Config::default() };
        let c = c.sanitized();
        assert_eq!(tabs(&c), vec![(1, vec![3]), (2, vec![5]), (4, vec![2])]);
        assert_eq!(c.tab(4, 2).unwrap().title, "Work");
        assert_eq!(c.clone().sanitized(), c);
        let c = Config { fences: vec![fence(1, &[(3, "")]), fence(2, &[(3, "")])], ..Config::default() }.sanitized();
        assert_eq!(tabs(&c), vec![(1, vec![3]), (2, vec![2])]);
    }

    #[test]
    fn huge_ids_never_wrap_to_zero() {
        let c = Config { fences: vec![fence(u64::MAX, &[(u64::MAX - 1, "")]), fence(1, &[(1, "")])], ..Config::default() };
        assert_eq!(c.next_id(), 2);
        let mut ids = c.ids(&[2, 3]);
        assert_eq!((ids.take(), ids.take()), (4, 5));
        let c = Config { fences: vec![fence(u64::MAX, &[(u64::MAX, ""), (u64::MAX, "")]), fence(9, &[(u64::MAX, "")])], ..c };
        let c = c.sanitized();
        assert_eq!(tabs(&c), vec![(u64::MAX, vec![u64::MAX]), (9, vec![9])]);
        assert_eq!(Config::default().next_id(), 1);
    }

    #[test]
    fn a_fence_keeps_at_most_64_tabs() {
        let many: Vec<(u64, &str)> = (1..=80).map(|id| (id, "")).collect();
        let c = Config { fences: vec![fence(1, &many), Fence::new(100, "B", (0, 0, 300, 200), Look::default())], ..Config::default() };
        let c = c.sanitized();
        assert_eq!((c.fences[0].tabs.len(), c.fences[1].tabs.len()), (TABS, 1));
        assert_eq!(c.next_id(), 101);
    }
}
