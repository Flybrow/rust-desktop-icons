use super::{
    Config,
    layouts::{Layout, place},
};
use serde::{Deserialize, Serialize};

pub type Rect = [i32; 4];
const KEPT: usize = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Adapt {
    Keep,
    Scale,
    #[default]
    Remember,
}

impl Adapt {
    pub const ALL: [Adapt; 3] = [Adapt::Keep, Adapt::Scale, Adapt::Remember];
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Screen {
    pub full: Rect,
    pub work: Rect,
}

pub fn same(a: &[Screen], b: &[Screen]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.full == y.full)
}

fn overlap(a: Rect, b: Rect) -> i64 {
    let (w, h) = (a[2].min(b[2]) - a[0].max(b[0]), a[3].min(b[3]) - a[1].max(b[1]));
    if w > 0 && h > 0 { w as i64 * h as i64 } else { 0 }
}

fn center(r: Rect) -> (i64, i64) {
    (((r[0] + r[2]) / 2) as i64, ((r[1] + r[3]) / 2) as i64)
}

fn distance(a: Rect, b: Rect) -> i64 {
    let ((ax, ay), (bx, by)) = (center(a), center(b));
    (ax - bx).pow(2) + (ay - by).pow(2)
}

fn closest(r: Rect, screens: &[Screen]) -> Option<usize> {
    let best = (0..screens.len()).max_by_key(|&i| overlap(r, screens[i].full)).filter(|&i| overlap(r, screens[i].full) > 0);
    best.or_else(|| (0..screens.len()).min_by_key(|&i| distance(r, screens[i].full)))
}

fn counterpart(old: &Screen, now: &[Screen]) -> Option<usize> {
    let primary = |s: &Screen| s.full[0] == 0 && s.full[1] == 0;
    now.iter()
        .position(|s| s.full == old.full)
        .or_else(|| primary(old).then(|| now.iter().position(primary)).flatten())
        .or_else(|| closest(old.full, now))
}

fn span(v: i32, len: i32, (a0, a1): (i32, i32), (b0, b1): (i32, i32)) -> (i32, i32) {
    let (wa, wb) = ((a1 - a0).max(1) as i64, (b1 - b0) as i64);
    let size = ((len as i64 * wb / wa) as i32).min(b1 - b0);
    let start = b0 + ((v - a0) as i64 * wb / wa) as i32;
    (start.clamp(b0, (b1 - size).max(b0)), size)
}

pub fn scale([x, y, w, h]: Rect, old: &[Screen], now: &[Screen]) -> Rect {
    let found = closest([x, y, x + w, y + h], old).and_then(|i| Some((old[i].work, now[counterpart(&old[i], now)?].work)));
    let Some((a, b)) = found.filter(|(a, b)| a != b) else { return [x, y, w, h] };
    let ((nx, nw), (ny, nh)) = (span(x, w, (a[0], a[2]), (b[0], b[2])), span(y, h, (a[1], a[3]), (b[1], b[3])));
    [nx, ny, nw, nh]
}

impl Config {
    pub fn adapt(&mut self, now: &[Screen]) -> Option<bool> {
        if now.is_empty() || self.screens == now {
            return None;
        }
        let old = std::mem::replace(&mut self.screens, now.to_vec());
        if old.is_empty() || same(&old, now) || self.screen_adapt == Adapt::Keep {
            return Some(false);
        }
        let saved = self.layouts.iter().position(|l| same(&l.screens, now)).map(|i| self.layouts.remove(i));
        let saved = saved.filter(|_| self.screen_adapt == Adapt::Remember);
        if self.screen_adapt == Adapt::Remember {
            self.layouts.retain(|l| !same(&l.screens, &old));
            self.layouts.push(Layout { screens: old.clone(), ..self.layout() });
            let extra = self.layouts.len().saturating_sub(KEPT);
            self.layouts.drain(..extra);
        }
        for f in &mut self.fences {
            let r = saved.as_ref().and_then(|l| l.rect(f.id)).unwrap_or_else(|| scale([f.x, f.y, f.w, f.h], &old, now));
            place(f, r);
        }
        Some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Fence;

    fn screen(full: Rect, taskbar: i32) -> Screen {
        Screen { full, work: [full[0], full[1], full[2], full[3] - taskbar] }
    }

    fn fence(id: u64, r: Rect) -> Fence {
        Fence { id, x: r[0], y: r[1], w: r[2], h: r[3], ..Fence::default() }
    }

    fn rects(c: &Config) -> Vec<Rect> {
        c.fences.iter().map(|f| [f.x, f.y, f.w, f.h]).collect()
    }

    #[test]
    fn scales_into_the_matching_screen() {
        let big = [screen([0, 0, 3840, 2160], 0)];
        let laptop = [screen([0, 0, 1920, 1080], 0)];
        assert_eq!(scale([2000, 1000, 800, 600], &big, &laptop), [1000, 500, 400, 300]);
        assert_eq!(scale([100, 100, 300, 200], &laptop, &laptop), [100, 100, 300, 200]);
        assert_eq!(scale([3500, 100, 300, 200], &big, &laptop), [1750, 50, 150, 100]);
        let wide = [screen([0, 0, 1920, 1080], 0)];
        assert_eq!(scale([1800, 0, 600, 100], &[screen([0, 0, 1920, 1080], 0)], &[screen([0, 0, 1280, 1080], 0)]), [880, 0, 400, 100]);
        assert_eq!(scale([0, 0, 2000, 100], &wide, &[screen([0, 0, 1000, 1080], 0)]), [0, 0, 1000, 100]);
    }

    #[test]
    fn unplugged_monitor_moves_its_fences_home() {
        let two = [screen([0, 0, 1920, 1080], 40), screen([1920, 0, 3840, 1080], 0)];
        let one = [screen([0, 0, 1920, 1080], 40)];
        assert_eq!(scale([2020, 100, 300, 200], &two, &one), [100, 96, 300, 192]);
        assert_eq!(scale([100, 100, 300, 200], &two, &one), [100, 100, 300, 200]);
        let moved = [screen([-1920, 0, 0, 1080], 0), screen([0, 0, 2560, 1440], 0)];
        assert_eq!(scale([100, 100, 300, 200], &one, &moved), [133, 138, 400, 276]);
    }

    #[test]
    fn layouts_follow_the_setting() {
        let (home, away) = (vec![screen([0, 0, 3840, 2160], 0)], vec![screen([0, 0, 1920, 1080], 0)]);
        let mut c = Config { fences: vec![fence(1, [2000, 1000, 800, 600])], ..Config::default() };
        assert_eq!((c.adapt(&home), c.adapt(&home)), (Some(false), None));
        assert_eq!(c.adapt(&away), Some(true));
        assert_eq!(rects(&c), vec![[1000, 500, 400, 300]]);
        c.fences[0].x = 10;
        c.fences.push(fence(2, [0, 0, 200, 200]));
        assert_eq!(c.adapt(&home), Some(true));
        assert_eq!(rects(&c), vec![[2000, 1000, 800, 600], [0, 0, 400, 400]]);
        assert_eq!(c.adapt(&away), Some(true));
        assert_eq!(rects(&c), vec![[10, 500, 400, 300], [0, 0, 200, 200]]);
        assert_eq!(c.layouts.len(), 1);
        c.screen_adapt = Adapt::Scale;
        assert_eq!(c.adapt(&home), Some(true));
        assert_eq!(rects(&c)[0], [20, 1000, 800, 600]);
        c.screen_adapt = Adapt::Keep;
        assert_eq!(c.adapt(&away), Some(false));
        assert_eq!(rects(&c)[0], [20, 1000, 800, 600]);
        let back: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!((back.screens, back.screen_adapt), (away, Adapt::Keep));
    }

    #[test]
    fn taskbar_moves_keep_the_layout() {
        let mut c = Config { fences: vec![fence(1, [100, 100, 300, 200])], ..Config::default() };
        c.adapt(&[screen([0, 0, 1920, 1080], 40)]);
        assert_eq!(c.adapt(&[screen([0, 0, 1920, 1080], 60)]), Some(false));
        assert_eq!(rects(&c), vec![[100, 100, 300, 200]]);
    }
}
