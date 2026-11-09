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

const STEP: usize = 16;

pub fn beside(f: &Fence, [ax, ay, aw, ah]: Rect, gap: i32, others: &[Rect]) -> (i32, i32) {
    let (w, h) = (f.w, f.h);
    let fits = |&(x, y): &(i32, i32)| x >= ax && y >= ay && x + w <= ax + aw && y + h <= ay + ah;
    let free = |&(x, y): &(i32, i32)| {
        others.iter().all(|&[ox, oy, ow, oh]| x + w + gap <= ox || ox + ow + gap <= x || y + h + gap <= oy || oy + oh + gap <= y)
    };
    let near = [(f.x + w + gap, f.y), (f.x - w - gap, f.y), (f.x, f.y + h + gap), (f.x, f.y - h - gap)];
    let dist = |&(x, y): &(i32, i32)| (x as i64 - f.x as i64).pow(2) + (y as i64 - f.y as i64).pow(2);
    let spots = (ay..=ay + ah - h).step_by(STEP).flat_map(|y| (ax..=ax + aw - w).step_by(STEP).map(move |x| (x, y)));
    near.into_iter()
        .filter(fits)
        .find(free)
        .or_else(|| spots.filter(free).min_by_key(dist))
        .or_else(|| near.into_iter().find(fits))
        .unwrap_or((f.x + gap, f.y + gap))
}

impl Config {
    pub fn duplicate(&mut self, id: u64, area: Rect, gap: i32) -> Option<Fence> {
        let src = self.fence(id)?.clone();
        let mut next = self.next_id();
        let mut fresh = || {
            next += 1;
            next - 1
        };
        let others: Vec<Rect> = self.fences.iter().map(|f| [f.x, f.y, f.w, f.h]).collect();
        let (x, y) = beside(&src, area, gap, &others);
        let tabs = src.tabs.iter().map(|t| super::Tab { id: fresh(), order: vec![], pins: vec![], system: vec![], ..t.clone() }).collect();
        let copy = Fence { id: fresh(), x, y, tabs, tab: 0, group: None, ..src };
        self.fences.push(copy.clone());
        Some(copy)
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
    use crate::domain::{Kind, Look, Tab, system::Sys};

    #[test]
    fn duplicate_copies_look_and_tabs_without_content() {
        let mut a = Fence::new(1, "Docs", (100, 100, 300, 200), Look { color: 0x123, ..Look::default() });
        a.tabs[0].kinds = vec![Kind::Images];
        a.tabs[0].order = vec!["x.txt".into()];
        a.tabs[0].system = vec![Sys::RecycleBin];
        a.tabs.push(Tab { id: 2, title: "Web".into(), portal: Some("/w".into()), ..Tab::default() });
        a.group = Some(4);
        let mut c = Config { fences: vec![a], ..Config::default() };
        let d = c.duplicate(1, [0, 0, 1000, 800], 10).unwrap();
        assert_eq!((d.id, d.x, d.y, d.w, d.h, d.look.color, d.group), (5, 410, 100, 300, 200, 0x123, None));
        assert_eq!(d.tabs.iter().map(|t| (t.id, t.title.as_str())).collect::<Vec<_>>(), vec![(3, "Docs"), (4, "Web")]);
        assert!(d.tabs[0].order.is_empty() && d.tabs[0].system.is_empty() && d.tabs[0].kinds == vec![Kind::Images]);
        assert_eq!((d.tabs[1].portal.as_deref(), c.fences.len(), c.next_id()), (Some(std::path::Path::new("/w")), 2, 6));
        assert_eq!(c.duplicate(9, [0, 0, 1000, 800], 10), None);
        let f = Fence { x: 700, y: 600, w: 300, h: 200, ..Fence::default() };
        let own = [700, 600, 300, 200];
        assert_eq!((beside(&f, [0, 0, 1000, 800], 10, &[own]), beside(&f, own, 10, &[own])), ((390, 600), (710, 610)));
    }

    #[test]
    fn duplicate_avoids_other_fences() {
        let f = Fence { x: 100, y: 100, w: 300, h: 200, ..Fence::default() };
        let (own, right) = ([100, 100, 300, 200], [410, 100, 300, 200]);
        let area = [0, 0, 1900, 1000];
        assert_eq!(beside(&f, area, 10, &[own, right]), (100, 310));
        let all = [own, right, [100, 310, 300, 200], [720, 0, 300, 600]];
        let p = beside(&f, area, 10, &all);
        assert!(all.iter().all(|&[x, y, w, h]| p.0 + 300 + 10 <= x || x + w + 10 <= p.0 || p.1 + 200 + 10 <= y || y + h + 10 <= p.1));
        assert_eq!(p, (96, 528));
        let full = [[0, 0, 1900, 1000]];
        assert_eq!(beside(&f, area, 10, &full), (410, 100));
    }

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

    #[test]
    fn a_copy_goes_left_then_below_then_above() {
        let f = Fence { x: 400, y: 400, w: 100, h: 100, ..Fence::default() };
        let (own, area) = ([400, 400, 100, 100], [0, 0, 1000, 1000]);
        let (right, left, below) = ([510, 400, 100, 100], [290, 400, 100, 100], [400, 510, 100, 100]);
        assert_eq!(beside(&f, area, 10, &[own, right]), (290, 400));
        assert_eq!(beside(&f, area, 10, &[own, right, left]), (400, 510));
        assert_eq!(beside(&f, area, 10, &[own, right, left, below]), (400, 290));
        let ring = [own, right, left, below, [400, 290, 100, 100]];
        let (x, y) = beside(&f, area, 10, &ring);
        assert!((x - 400).abs() <= 250 && (y - 400).abs() <= 250 && (x, y) != (400, 400), "{x} {y}");
        let mut a = Fence::new(1, "A", (400, 400, 100, 100), Look::default());
        a.tabs.push(Tab { id: 2, pins: vec![crate::domain::pins::Pin { path: "x".into(), id: 1 }], ..Tab::default() });
        a.tab = 1;
        let mut c = Config {
            fences: vec![a, Fence { id: 3, x: 510, y: 400, w: 100, h: 100, ..Fence::new(3, "B", (510, 400, 100, 100), Look::default()) }],
            ..Config::default()
        };
        c.fences.push(Fence::new(4, "C", (290, 400, 100, 100), Look::default()));
        let d = c.duplicate(1, area, 10).unwrap();
        assert_eq!((d.x, d.y, d.tab), (400, 510, 0));
        assert!(d.tabs.iter().all(|t| t.pins.is_empty()));
        c.snapshot("A".into());
        c.forget_snapshot(1);
        assert_eq!(c.snapshots.len(), 1);
    }
}
