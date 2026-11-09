pub type Rect = [i32; 4];
const MAX_GAP: i32 = 400;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guide {
    pub from: (i32, i32),
    pub to: (i32, i32),
}

fn flip(r: Rect) -> Rect {
    [r[1], r[0], r[3], r[2]]
}

fn flip_guide(g: Guide) -> Guide {
    Guide { from: (g.from.1, g.from.0), to: (g.to.1, g.to.0) }
}

fn across(a: Rect, b: Rect) -> bool {
    a[1] < b[3] && b[1] < a[3]
}

fn gaps(others: &[Rect], gap: i32) -> Vec<i32> {
    let mut out = vec![gap];
    for a in others {
        out.extend(others.iter().filter(|b| across(*a, **b)).map(|b| b[0] - a[2]).filter(|g| (1..=MAX_GAP).contains(g)));
    }
    out
}

fn shift(r: Rect, area: Rect, others: &[Rect], gaps: &[i32], d: i32) -> i32 {
    let (mut lo, mut hi) = (vec![area[0]], vec![area[2]]);
    for o in others {
        lo.extend([o[0], o[2]]);
        hi.extend([o[0], o[2]]);
        if across(r, *o) {
            lo.extend(gaps.iter().map(|g| o[2] + g));
            hi.extend(gaps.iter().map(|g| o[0] - g));
        }
    }
    lo.iter().map(|t| t - r[0]).chain(hi.iter().map(|t| t - r[2])).filter(|v| v.abs() < d).min_by_key(|v| v.abs()).unwrap_or(0)
}

fn lines(r: Rect, others: &[Rect], gaps: &[i32]) -> Vec<Guide> {
    let mut out = vec![];
    for o in others {
        for e in [r[0], r[2]].into_iter().filter(|e| *e == o[0] || *e == o[2]) {
            out.push(Guide { from: (e, r[1].min(o[1])), to: (e, r[3].max(o[3])) });
        }
    }
    let mid = |o: &Rect| (r[1].max(o[1]) + r[3].min(o[3])) / 2;
    let left = others.iter().filter(|o| across(r, **o) && o[2] < r[0]).max_by_key(|o| o[2]);
    let right = others.iter().filter(|o| across(r, **o) && o[0] > r[2]).min_by_key(|o| o[0]);
    out.extend(left.filter(|o| gaps.contains(&(r[0] - o[2]))).map(|o| Guide { from: (o[2], mid(o)), to: (r[0], mid(o)) }));
    out.extend(right.filter(|o| gaps.contains(&(o[0] - r[2]))).map(|o| Guide { from: (r[2], mid(o)), to: (o[0], mid(o)) }));
    out
}

pub fn guides(r: Rect, others: &[Rect], gap: i32) -> Vec<Guide> {
    let flipped: Vec<Rect> = others.iter().map(|o| flip(*o)).collect();
    let mut out = lines(r, others, &gaps(others, gap));
    out.extend(lines(flip(r), &flipped, &gaps(&flipped, gap)).into_iter().map(flip_guide));
    out
}

pub fn moving(r: Rect, area: Rect, others: &[Rect], d: i32, gap: i32) -> (Rect, Vec<Guide>) {
    let flipped: Vec<Rect> = others.iter().map(|o| flip(*o)).collect();
    let dx = shift(r, area, others, &gaps(others, gap), d);
    let dy = shift(flip(r), flip(area), &flipped, &gaps(&flipped, gap), d);
    let out = [r[0] + dx, r[1] + dy, r[2] + dx, r[3] + dy];
    (out, guides(out, others, gap))
}

pub fn follow(start: Rect, (x0, y0): (i32, i32), (x, y): (i32, i32)) -> Rect {
    let (dx, dy) = (x - x0, y - y0);
    [start[0] + dx, start[1] + dy, start[2] + dx, start[3] + dy]
}

pub fn sizing(r: Rect, area: Rect, d: i32, sides: [bool; 4]) -> Rect {
    std::array::from_fn(|i| if sides[i] && (area[i] - r[i]).abs() < d { area[i] } else { r[i] })
}

pub fn grid_len(len: i32, chrome: i32, cell: i32, min: i32, room: i32) -> i32 {
    let cell = cell.max(1);
    let n = ((len - chrome + cell / 2) / cell).max(1);
    let n = if chrome + n * cell > room { ((room - chrome) / cell).max(1) } else { n };
    (chrome + n * cell).max(min)
}

pub fn resize(r: Rect, sides: [bool; 4], (w, h): (Option<i32>, Option<i32>)) -> Rect {
    let mut out = r;
    for (lo, len) in [(0, w), (1, h)] {
        match len {
            Some(n) if sides[lo] => out[lo] = r[lo + 2] - n,
            Some(n) if sides[lo + 2] => out[lo + 2] = r[lo] + n,
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = [0, 0, 1920, 1040];

    fn snap(r: Rect, others: &[Rect]) -> Rect {
        moving(r, AREA, others, 16, 10).0
    }

    #[test]
    fn slow_drag_leaves_the_edge() {
        let start = [0, 0, 300, 150];
        let mut r = start;
        for step in 1..=20 {
            r = snap(follow(start, (150, 17), (150 + step * 12, 17 + step * 10)), &[]);
        }
        assert_eq!(r, [240, 200, 540, 350]);
        assert_eq!(snap(follow(start, (150, 17), (160, 27)), &[]), start);
    }

    #[test]
    fn snapping() {
        assert_eq!(snap([5, 300, 365, 540], &[]), [0, 300, 360, 540]);
        assert_eq!(snap([-7, 1030, 353, 1050], &[]), [0, 1020, 360, 1040]);
        assert_eq!(snap([1565, 10, 1925, 250], &[]), [1560, 0, 1920, 240]);
        assert_eq!(snap([100, 100, 460, 340], &[]), [100, 100, 460, 340]);
        assert_eq!(sizing([4, 4, 1910, 1045], AREA, 16, [false, true, true, false]), [4, 0, 1920, 1045]);
        assert_eq!(sizing([4, 4, 1910, 1045], AREA, 16, [true; 4]), AREA);
    }

    #[test]
    fn lines_up_with_other_fences() {
        let other = [500, 500, 800, 700];
        let (r, g) = moving([505, 100, 705, 300], AREA, &[other], 16, 10);
        assert_eq!(r, [500, 100, 700, 300]);
        assert_eq!(g, vec![Guide { from: (500, 100), to: (500, 700) }]);
        let (r, g) = moving([812, 530, 1012, 660], AREA, &[other], 16, 10);
        assert_eq!(r, [810, 530, 1010, 660]);
        assert_eq!(g, vec![Guide { from: (800, 595), to: (810, 595) }]);
        assert_eq!(snap([830, 100, 1030, 300], &[other]), [830, 100, 1030, 300]);
    }

    #[test]
    fn matches_existing_spacing() {
        let (a, b) = ([100, 100, 300, 300], [340, 100, 540, 300]);
        let (r, g) = moving([575, 103, 775, 303], AREA, &[a, b], 16, 10);
        assert_eq!(r, [580, 100, 780, 300]);
        assert!(g.contains(&Guide { from: (540, 200), to: (580, 200) }));
        assert!(g.contains(&Guide { from: (100, 100), to: (780, 100) }));
    }

    #[test]
    fn sizes_follow_the_icon_grid() {
        assert_eq!(grid_len(250, 0, 80, 120, 2000), 240);
        assert_eq!(grid_len(285, 0, 80, 120, 2000), 320);
        assert_eq!(grid_len(50, 0, 80, 120, 2000), 120);
        assert_eq!(grid_len(300, 46, 80, 80, 2000), 286);
        assert_eq!(grid_len(330, 0, 80, 120, 300), 240);
        assert_eq!(resize([100, 100, 350, 300], [false, false, true, true], (Some(240), Some(206))), [100, 100, 340, 306]);
        assert_eq!(resize([100, 100, 350, 300], [true, true, false, false], (Some(240), None)), [110, 100, 350, 300]);
    }

    #[test]
    fn spacing_only_counts_fences_side_by_side() {
        let other = [500, 500, 800, 700];
        assert_eq!(snap([812, 100, 1012, 300], &[other]), [800, 100, 1000, 300]);
        assert_eq!(snap([812, 300, 1012, 500], &[other]), [800, 300, 1000, 500]);
        assert_eq!(snap([812, 690, 1012, 890], &[other]), [810, 700, 1010, 900]);
        assert_eq!(snap([290, 550, 488, 650], &[other]), [292, 550, 490, 650]);
        assert_eq!((snap([16, 300, 376, 540], &[]), snap([15, 300, 375, 540], &[])), ([16, 300, 376, 540], [0, 300, 360, 540]));
        let (a, b) = ([400, 100, 600, 300], [640, 100, 840, 300]);
        let (r, g) = moving([160, 100, 360, 300], AREA, &[a, b], 16, 10);
        assert_eq!(r, [160, 100, 360, 300]);
        assert!(g.contains(&Guide { from: (360, 200), to: (400, 200) }), "{g:?}");
        assert_eq!(sizing([100, 100, 500, 500], AREA, 16, [true; 4]), [100, 100, 500, 500]);
        assert_eq!(sizing([16, 100, 1904, 500], AREA, 16, [true; 4]), [16, 100, 1904, 500]);
        assert_eq!(grid_len(330, 46, 80, 80, 300), 286);
        assert_eq!(resize([100, 100, 350, 300], [false; 4], (Some(240), Some(100))), [100, 100, 350, 300]);
    }

    #[test]
    fn touching_fences_are_not_side_by_side() {
        let other = [500, 500, 800, 700];
        assert_eq!(snap([812, 700, 1012, 900], &[other]), [800, 700, 1000, 900]);
        let (l, a) = ([0, 100, 100, 300], [370, 100, 570, 300]);
        let (r, g) = moving([160, 100, 360, 300], AREA, &[l, a], 16, 10);
        assert_eq!(r, [160, 100, 360, 300]);
        assert!(g.contains(&Guide { from: (360, 200), to: (370, 200) }), "{g:?}");
    }
}
