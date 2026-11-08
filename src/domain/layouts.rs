use super::{
    Config, Fence,
    model::clean,
    screens::{Rect, Screen, same, scale},
};
use serde::{Deserialize, Serialize};

pub const SNAPSHOTS: usize = 10;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub screens: Vec<Screen>,
    pub fences: Vec<(u64, Rect)>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Snapshot {
    pub name: String,
    pub layout: Layout,
}

impl Layout {
    pub fn rect(&self, id: u64) -> Option<Rect> {
        self.fences.iter().find(|(f, _)| *f == id).map(|(_, r)| *r)
    }
}

pub fn place(f: &mut Fence, [x, y, w, h]: Rect) {
    (f.x, f.y, f.w, f.h) = (x, y, w.max(Fence::MIN.0), h.max(Fence::MIN.1));
}

impl Config {
    pub fn layout(&self) -> Layout {
        Layout { screens: self.screens.clone(), fences: self.fences.iter().map(|f| (f.id, [f.x, f.y, f.w, f.h])).collect() }
    }

    pub fn snapshot(&mut self, name: String) {
        let layout = self.layout();
        self.snapshots.push(Snapshot { name, layout });
        let extra = self.snapshots.len().saturating_sub(SNAPSHOTS);
        self.snapshots.drain(..extra);
    }

    pub fn restore(&mut self, i: usize) -> bool {
        let Some(l) = self.snapshots.get(i).map(|s| s.layout.clone()) else { return false };
        let moved = !l.screens.is_empty() && !self.screens.is_empty() && !same(&l.screens, &self.screens);
        for f in &mut self.fences {
            if let Some(r) = l.rect(f.id) {
                place(f, if moved { scale(r, &l.screens, &self.screens) } else { r });
            }
        }
        true
    }

    pub fn rename_snapshot(&mut self, i: usize, name: &str) {
        let name = clean(name.trim());
        if let Some(s) = self.snapshots.get_mut(i).filter(|_| !name.is_empty()) {
            s.name = name;
        }
    }

    pub fn forget_snapshot(&mut self, i: usize) {
        if i < self.snapshots.len() {
            self.snapshots.remove(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fence(id: u64, x: i32) -> Fence {
        Fence { id, x, y: 10, w: 300, h: 200, ..Fence::default() }
    }

    #[test]
    fn snapshots_restore_known_fences() {
        let mut c = Config { fences: vec![fence(1, 100), fence(2, 500)], ..Config::default() };
        c.snapshot("A".into());
        c.fences[0].x = 900;
        c.fences.push(fence(3, 50));
        assert!(c.restore(0));
        assert_eq!(c.fences.iter().map(|f| f.x).collect::<Vec<_>>(), vec![100, 500, 50]);
        assert!(!c.restore(4));
        for k in 0..12 {
            c.snapshot(format!("S{k}"));
        }
        assert_eq!((c.snapshots.len(), c.snapshots[0].name.as_str()), (SNAPSHOTS, "S2"));
        c.forget_snapshot(0);
        c.forget_snapshot(99);
        assert_eq!(c.snapshots[0].name, "S3");
        c.rename_snapshot(0, "  Bureau\tmaison  ");
        c.rename_snapshot(0, "   ");
        c.rename_snapshot(42, "x");
        assert_eq!(c.snapshots[0].name, "Bureaumaison");
        let back: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back.snapshots, c.snapshots);
    }

    #[test]
    fn snapshots_follow_the_screen() {
        let big = Screen { full: [0, 0, 3840, 2160], work: [0, 0, 3840, 2160] };
        let small = Screen { full: [0, 0, 1920, 1080], work: [0, 0, 1920, 1080] };
        let mut c = Config { fences: vec![fence(1, 2000)], screens: vec![big], ..Config::default() };
        c.snapshot("big".into());
        c.screens = vec![small];
        c.restore(0);
        assert_eq!((c.fences[0].x, c.fences[0].w), (1000, 150));
    }
}
