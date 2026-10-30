use super::{Config, Tab};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pin {
    pub path: PathBuf,
    pub id: u64,
}

fn same(a: &Path, b: &Path) -> bool {
    a.as_os_str().eq_ignore_ascii_case(b.as_os_str())
}

pub fn reconcile(pins: &[Pin], entries: &[(PathBuf, u64)]) -> Vec<Pin> {
    let mut out: Vec<Pin> = vec![];
    for p in pins {
        let by_path = entries.iter().find(|(e, _)| same(e, &p.path));
        let found = by_path.or_else(|| entries.iter().find(|(_, id)| p.id != 0 && *id == p.id));
        if let Some((path, id)) = found.filter(|(e, _)| !out.iter().any(|o| same(&o.path, e))) {
            out.push(Pin { path: path.clone(), id: *id });
        }
    }
    out
}

impl Tab {
    pub fn pinned(&self, p: &Path) -> bool {
        self.pins.iter().any(|x| same(&x.path, p))
    }
}

impl Config {
    pub fn pinned(&self) -> Vec<PathBuf> {
        self.fences.iter().flat_map(|f| &f.tabs).flat_map(|t| &t.pins).map(|p| p.path.clone()).collect()
    }

    pub fn is_pinned(&self, p: &Path) -> bool {
        self.fences.iter().flat_map(|f| &f.tabs).any(|t| t.pinned(p))
    }

    pub fn unpin(&mut self, paths: &[PathBuf]) {
        for t in self.fences.iter_mut().flat_map(|f| f.tabs.iter_mut()) {
            t.pins.retain(|p| !paths.iter().any(|q| same(q, &p.path)));
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

pub fn free_slots(taken: &[(i32, i32)], cell: (i32, i32), (ox, oy, _, h): (i32, i32, i32, i32), n: usize) -> Vec<(i32, i32)> {
    let (cw, ch) = (cell.0.max(1), cell.1.max(1));
    let rows = (h / ch).max(1);
    let busy = |x: i32, y: i32| taken.iter().any(|&(tx, ty)| (tx - x).abs() < cw && (ty - y).abs() < ch);
    (0..).map(|i| (ox + i / rows * cw, oy + i % rows * ch)).filter(|&(x, y)| !busy(x, y)).take(n).collect()
}

pub fn corner_slot(taken: &[(i32, i32)], cell: (i32, i32), (ox, oy, w, h): (i32, i32, i32, i32)) -> (i32, i32) {
    let (cw, ch) = (cell.0.max(1), cell.1.max(1));
    let rows = (h / ch).max(1);
    let busy = |x: i32, y: i32| taken.iter().any(|&(tx, ty)| (tx - x).abs() < cw && (ty - y).abs() < ch);
    (0..rows * (w / cw).max(1))
        .map(|i| (ox + w - (i / rows + 1) * cw, oy + h - (i % rows + 1) * ch))
        .find(|&(x, y)| !busy(x, y))
        .unwrap_or((ox + w - cw, oy + h - ch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Fence, Look};

    fn pin(p: &str, id: u64) -> Pin {
        Pin { path: p.into(), id }
    }

    #[test]
    fn reconcile_follows_renames_and_drops_deleted() {
        let pins = vec![pin("D:\\a.txt", 1), pin("D:\\b.txt", 2), pin("D:\\c.txt", 3), pin("D:\\x.txt", 0)];
        let entries = vec![("D:\\A.TXT".into(), 1), ("D:\\renamed.txt".into(), 2), ("D:\\other.txt".into(), 9)];
        assert_eq!(reconcile(&pins, &entries), vec![pin("D:\\A.TXT", 1), pin("D:\\renamed.txt", 2)]);
        assert_eq!(reconcile(&[pin("D:\\a.txt", 1), pin("D:\\A.txt", 1)], &[("D:\\a.txt".into(), 1)]).len(), 1);
    }

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
    fn slots_fill_columns_and_skip_taken() {
        assert_eq!(free_slots(&[], (100, 100), (0, 0, 1000, 250), 3), vec![(0, 0), (0, 100), (100, 0)]);
        assert_eq!(free_slots(&[(0, 5), (0, 100)], (100, 100), (0, 0, 1000, 200), 2), vec![(100, 0), (100, 100)]);
        assert_eq!(free_slots(&[], (0, 0), (0, 0, 0, 0), 2), vec![(0, 0), (1, 0)]);
        assert_eq!(free_slots(&[(1920, 0)], (100, 100), (1920, 0, 1000, 200), 2), vec![(1920, 100), (2020, 0)]);
    }

    #[test]
    fn hidden_stack_sits_in_the_bottom_right_free_cell() {
        let area = (1920, 0, 1000, 1000);
        assert_eq!(corner_slot(&[], (100, 100), area), (2820, 900));
        assert_eq!(corner_slot(&[(2830, 905)], (100, 100), area), (2820, 800));
        let full: Vec<(i32, i32)> = (0..10).map(|r| (2820, r * 100)).collect();
        assert_eq!(corner_slot(&full, (100, 100), area), (2720, 900));
        assert_eq!(corner_slot(&[], (0, 0), (0, 0, 0, 0)), (-1, -1));
    }
}
