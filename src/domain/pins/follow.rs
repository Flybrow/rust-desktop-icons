use super::{Pin, key};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

pub const GRACE_MS: u64 = 2000;

#[derive(Debug, Default)]
pub struct Missing(HashMap<String, u64>);

impl Missing {
    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn any(&self) -> bool {
        !self.0.is_empty()
    }

    pub fn prune<'a>(&mut self, pins: impl IntoIterator<Item = &'a Pin>) {
        let live: HashSet<String> = pins.into_iter().map(|p| key(&p.path)).collect();
        self.0.retain(|k, _| live.contains(k));
    }

    fn lasted(&mut self, k: &str, now: u64) -> bool {
        let since = *self.0.entry(k.to_string()).or_insert(now);
        now.saturating_sub(since) >= GRACE_MS
    }
}

pub fn temp(p: &Path) -> bool {
    p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('~') && n.to_ascii_lowercase().ends_with(".tmp"))
}

pub struct Listing {
    paths: HashMap<String, (PathBuf, u64)>,
    ids: HashMap<u64, PathBuf>,
}

impl Listing {
    pub fn new(entries: Vec<(PathBuf, u64)>) -> Self {
        let mut ids = HashMap::new();
        for (p, id) in entries.iter().filter(|(p, id)| *id != 0 && !temp(p)) {
            ids.entry(*id).or_insert_with(|| p.clone());
        }
        Self { paths: entries.into_iter().map(|(p, id)| (key(&p), (p, id))).collect(), ids }
    }

    pub fn follow(&self, pins: &[Pin], missing: &mut Missing, now: u64) -> (Vec<Pin>, Vec<PathBuf>) {
        let (mut out, mut seen, mut held) = (vec![], HashSet::new(), vec![]);
        for p in pins {
            let k = key(&p.path);
            let moved = self.ids.get(&p.id).filter(|_| p.id != 0);
            let found = match self.paths.get(&k) {
                Some(e) => {
                    missing.0.remove(&k);
                    Some(e.clone())
                }
                None if !missing.lasted(&k, now) => {
                    held.extend(moved.cloned());
                    Some((p.path.clone(), p.id))
                }
                None => {
                    missing.0.remove(&k);
                    moved.map(|m| (m.clone(), p.id))
                }
            };
            if let Some((path, id)) = found.filter(|(path, _)| seen.insert(key(path))) {
                out.push(Pin { path, id });
            }
        }
        (out, held)
    }
}

#[cfg(test)]
mod tests {
    use super::{super::pin, *};

    fn listing(e: &[(&str, u64)]) -> Listing {
        Listing::new(e.iter().map(|(p, id)| (PathBuf::from(p), *id)).collect())
    }

    #[test]
    fn present_pins_follow_the_listing_case_and_id() {
        let mut m = Missing::default();
        let l = listing(&[("D:\\A.TXT", 7), ("D:\\other.txt", 9)]);
        assert_eq!(l.follow(&[pin("D:\\a.txt", 1)], &mut m, 0), (vec![pin("D:\\A.TXT", 7)], vec![]));
        assert_eq!(l.follow(&[pin("D:\\a.txt", 1), pin("D:\\A.txt", 1)], &mut m, 0).0.len(), 1);
    }

    #[test]
    fn a_missing_pin_waits_for_a_second_scan_before_moving_or_going() {
        let mut m = Missing::default();
        let pins = [pin("D:\\b.txt", 2), pin("D:\\c.txt", 3), pin("D:\\x.txt", 0)];
        let l = listing(&[("D:\\renamed.txt", 2)]);
        let (kept, held) = l.follow(&pins, &mut m, 1000);
        assert_eq!((kept, held), (pins.to_vec(), vec![PathBuf::from("D:\\renamed.txt")]), "first scan: nothing changes");
        assert_eq!(l.follow(&pins, &mut m, 1000 + GRACE_MS - 1).0, pins.to_vec(), "too soon");
        assert_eq!(l.follow(&pins, &mut m, 1000 + GRACE_MS).0, vec![pin("D:\\renamed.txt", 2)]);
        assert_eq!(l.follow(&pins, &mut m, 1000 + GRACE_MS + 1).0, pins.to_vec(), "a new absence starts a new wait");
    }

    #[test]
    fn a_pin_back_in_time_is_kept_and_forgets_its_absence() {
        let mut m = Missing::default();
        let doc = [pin("D:\\doc.docx", 5)];
        let saving = listing(&[("D:\\~WRL0001.tmp", 5)]);
        assert_eq!(saving.follow(&doc, &mut m, 0), (doc.to_vec(), vec![]), "never follows an Office temp name");
        assert_eq!(listing(&[("D:\\doc.docx", 6)]).follow(&doc, &mut m, 10).0, vec![pin("D:\\doc.docx", 6)]);
        assert_eq!(saving.follow(&doc, &mut m, GRACE_MS).0, doc.to_vec(), "the absence was forgotten");
        assert!(saving.follow(&doc, &mut m, GRACE_MS * 2).0.is_empty());
    }

    #[test]
    fn absences_of_unpinned_paths_are_pruned() {
        let mut m = Missing::default();
        let gone = [pin("D:\\gone.txt", 0)];
        listing(&[]).follow(&gone, &mut m, 0);
        m.prune(&[]);
        assert_eq!(listing(&[]).follow(&gone, &mut m, GRACE_MS).0, gone.to_vec(), "pruned: the wait starts again");
        assert!(m.any());
        m.clear();
        assert!(!m.any());
        assert!(temp(Path::new("C:\\d\\~WRD0003.TMP")) && !temp(Path::new("C:\\d\\a.tmp")) && !temp(Path::new("C:\\d\\~a.txt")));
    }
}
