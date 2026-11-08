use super::{Config, Fence, Tab};

impl Config {
    pub fn orphans(&self, dirs: &[u64]) -> Vec<u64> {
        let known: Vec<u64> = self.fences.iter().flat_map(|f| &f.tabs).map(|t| t.id).collect();
        let mut out: Vec<u64> = dirs.iter().copied().filter(|d| *d > 0 && !known.contains(d)).collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    pub fn recover(&mut self, dirs: &[u64], title: &str, at: (i32, i32)) -> Option<u64> {
        let orphans = self.orphans(dirs);
        if orphans.is_empty() {
            return None;
        }
        let id = self.next_id().max(orphans.iter().max().map_or(0, |m| m + 1));
        let name = |k: usize| if k == 0 { title.to_string() } else { format!("{title} {}", k + 1) };
        let tabs = orphans.iter().enumerate().map(|(k, &d)| Tab { id: d, title: name(k), ..Tab::default() }).collect();
        let look = self.look.clone();
        self.fences.push(Fence { id, x: at.0, y: at.1, tabs, custom: Some(vec![]), look, ..Fence::default() });
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orphan_folders_come_back_in_one_fence() {
        let mut c = Config { fences: vec![Fence::new(1, "A", (0, 0, 300, 200), Default::default())], ..Config::default() };
        c.fences[0].tabs.push(Tab { id: 7, portal: Some("C:\\x".into()), ..Tab::default() });
        assert_eq!(c.orphans(&[1, 0]), Vec::<u64>::new());
        assert_eq!(c.recover(&[1], "Recovered", (10, 20)), None);
        assert_eq!(c.orphans(&[9, 7, 3, 9, 1]), vec![3, 9]);
        let id = c.recover(&[9, 7, 3, 1], "Recovered", (10, 20)).unwrap();
        let f = c.fence(id).unwrap().clone();
        assert_eq!((id, f.x, f.y, f.look.clone()), (10, 10, 20, c.look.clone()));
        assert_eq!(f.tabs.iter().map(|t| (t.id, t.title.as_str())).collect::<Vec<_>>(), vec![(3, "Recovered"), (9, "Recovered 2")]);
        assert!(c.orphans(&[9, 7, 3, 1]).is_empty() && c.next_id() == 11);
        assert_eq!(c.clone().sanitized(), c);
    }

    #[test]
    fn the_recovered_fence_follows_the_default_look() {
        let mut c = Config::default();
        c.look.color = 0x123456;
        let id = c.recover(&[4], "R", (0, 0)).unwrap();
        assert_eq!(c.fence(id).unwrap().look.color, 0x123456);
    }
}
