use super::zone::Zone;

pub type Rect = (i32, i32, i32, i32);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Side {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    pub fn pick(vertical: bool, far: bool) -> Side {
        match (vertical, far) {
            (false, false) => Side::Top,
            (false, true) => Side::Bottom,
            (true, false) => Side::Left,
            (true, true) => Side::Right,
        }
    }

    pub fn vertical(self) -> bool {
        matches!(self, Side::Left | Side::Right)
    }

    pub fn far(self) -> bool {
        matches!(self, Side::Bottom | Side::Right)
    }

    pub fn flipped(self) -> bool {
        matches!(self, Side::Bottom | Side::Left)
    }

    pub fn zone(self, z: Zone) -> Zone {
        use Zone::*;
        let [top, bottom, left, right, tl, tr, bl, br] = match self {
            Side::Top => return z,
            Side::Bottom => [Bottom, Top, Left, Right, BottomLeft, BottomRight, TopLeft, TopRight],
            Side::Left => [Left, Right, Top, Bottom, TopLeft, BottomLeft, TopRight, BottomRight],
            Side::Right => [Right, Left, Top, Bottom, TopRight, BottomRight, TopLeft, BottomLeft],
        };
        match z {
            Top => top,
            Bottom => bottom,
            Left => left,
            Right => right,
            TopLeft => tl,
            TopRight => tr,
            BottomLeft => bl,
            BottomRight => br,
            z => z,
        }
    }
}

pub fn far(pos: i32, bar: i32, natural: i32, (lo, hi): (i32, i32), margin: i32) -> bool {
    let near = hi - pos - margin;
    let away = pos + bar - lo - margin;
    natural > near && away > near
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geo {
    pub side: Side,
    pub size: (i32, i32),
    pub bar: i32,
    pub gap: i32,
}

impl Geo {
    pub fn bar_rect(&self) -> Rect {
        let ((w, h), t) = (self.size, self.bar);
        match self.side {
            Side::Top => (0, 0, w, t),
            Side::Bottom => (0, h - t, w, h),
            Side::Left => (0, 0, t, h),
            Side::Right => (w - t, 0, w, h),
        }
    }

    pub fn body(&self) -> Rect {
        let ((w, h), t) = (self.size, self.bar);
        match self.side {
            Side::Top => (0, t, w, h),
            Side::Bottom => (0, 0, w, h - t),
            Side::Left => (t, 0, w, h),
            Side::Right => (0, 0, w - t, h),
        }
    }

    pub fn length(&self) -> i32 {
        if self.side.vertical() { self.size.1 } else { self.size.0 }
    }

    pub fn grid_w(&self) -> i32 {
        let (x0, _, x1, _) = self.body();
        if self.side.vertical() { x1 - x0 - self.gap } else { x1 - x0 }
    }

    pub fn view_h(&self) -> i32 {
        let (_, y0, _, y1) = self.body();
        if self.side.vertical() { y1 - y0 } else { y1 - y0 - self.gap }
    }

    pub fn to_window(self, (x, y): (i32, i32)) -> (i32, i32) {
        let ((x0, y0, x1, y1), g) = (self.body(), self.gap);
        match self.side {
            Side::Top => (x0 + x, y0 + g + y),
            Side::Bottom => (x0 + x, y1 - g - y),
            Side::Left => (x0 + g + x, y0 + y),
            Side::Right => (x1 - g - x, y0 + y),
        }
    }

    pub fn to_content(self, (px, py): (i32, i32)) -> (i32, i32) {
        let ((x0, y0, x1, y1), g) = (self.body(), self.gap);
        match self.side {
            Side::Top => (px - x0, py - y0 - g),
            Side::Bottom => (px - x0, y1 - g - py),
            Side::Left => (px - x0 - g, py - y0),
            Side::Right => (x1 - g - px, py - y0),
        }
    }

    pub fn rect(&self, (x0, y0, x1, y1): Rect) -> Rect {
        let ((ax, ay), (bx, by)) = (self.to_window((x0, y0)), self.to_window((x1, y1)));
        (ax.min(bx), ay.min(by), ax.max(bx), ay.max(by))
    }

    pub fn cell(&self, (x, y): (i32, i32), size: i32) -> (i32, i32) {
        let (l, t, _, _) = self.rect((x, y, x + size, y + size));
        (l, t)
    }

    pub fn along_bar(&self, (x, y): (i32, i32)) -> Option<i32> {
        let (x0, y0, x1, y1) = self.bar_rect();
        ((x0..x1).contains(&x) && (y0..y1).contains(&y)).then_some(if self.side.vertical() { y - y0 } else { x - x0 })
    }

    pub fn canonical(&self, (x, y): (i32, i32)) -> ((i32, i32), (i32, i32)) {
        let (w, h) = self.size;
        match self.side {
            Side::Top => ((w, h), (x, y)),
            Side::Bottom => ((w, h), (x, h - 1 - y)),
            Side::Left => ((h, w), (y, x)),
            Side::Right => ((h, w), (y, w - 1 - x)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geo(side: Side) -> Geo {
        Geo { side, size: (300, 200), bar: 30, gap: 4 }
    }

    #[test]
    fn bar_and_body_split_the_window() {
        assert_eq!((geo(Side::Top).bar_rect(), geo(Side::Top).body()), ((0, 0, 300, 30), (0, 30, 300, 200)));
        assert_eq!((geo(Side::Bottom).bar_rect(), geo(Side::Bottom).body()), ((0, 170, 300, 200), (0, 0, 300, 170)));
        assert_eq!((geo(Side::Left).bar_rect(), geo(Side::Left).body()), ((0, 0, 30, 200), (30, 0, 300, 200)));
        assert_eq!((geo(Side::Right).bar_rect(), geo(Side::Right).body()), ((270, 0, 300, 200), (0, 0, 270, 200)));
        assert_eq!((geo(Side::Left).length(), geo(Side::Left).grid_w(), geo(Side::Left).view_h()), (200, 266, 200));
        assert_eq!((geo(Side::Top).length(), geo(Side::Top).grid_w(), geo(Side::Top).view_h()), (300, 300, 166));
    }

    #[test]
    fn content_round_trips_and_hugs_the_bar() {
        for side in [Side::Top, Side::Bottom, Side::Left, Side::Right] {
            let g = geo(side);
            for p in [(0, 0), (17, 42), (120, 90)] {
                assert_eq!(g.to_content(g.to_window(p)), p, "{side:?}");
            }
        }
        assert_eq!(geo(Side::Bottom).cell((0, 0), 50), (0, 116));
        assert_eq!(geo(Side::Right).cell((0, 0), 50), (216, 0));
        assert_eq!(geo(Side::Left).cell((50, 50), 50), (84, 50));
        assert_eq!(geo(Side::Top).rect((0, 0, 10, 10)), (0, 34, 10, 44));
    }

    #[test]
    fn tabs_follow_the_bar() {
        assert_eq!(geo(Side::Top).along_bar((120, 10)), Some(120));
        assert_eq!(geo(Side::Top).along_bar((120, 40)), None);
        assert_eq!(geo(Side::Bottom).along_bar((5, 185)), Some(5));
        assert_eq!(geo(Side::Left).along_bar((10, 150)), Some(150));
        assert_eq!(geo(Side::Right).along_bar((280, 20)), Some(20));
        assert_eq!(geo(Side::Right).along_bar((10, 20)), None);
    }

    #[test]
    fn hit_zones_map_back_to_screen_edges() {
        use Zone::*;
        let g = geo(Side::Right);
        assert_eq!(g.canonical((299, 10)), ((200, 300), (10, 0)));
        assert_eq!((Side::Right.zone(Top), Side::Right.zone(Bottom), Side::Right.zone(Left)), (Right, Left, Top));
        assert_eq!((Side::Left.zone(Top), Side::Left.zone(TopRight), Side::Left.zone(BottomLeft)), (Left, BottomLeft, TopRight));
        assert_eq!((Side::Bottom.zone(Top), Side::Bottom.zone(TopLeft), Side::Bottom.zone(Caption)), (Bottom, BottomLeft, Caption));
        assert_eq!(Side::Top.zone(TopRight), TopRight);
        assert_eq!(geo(Side::Bottom).canonical((5, 199)), ((300, 200), (5, 0)));
        assert_eq!(geo(Side::Left).canonical((3, 150)), ((200, 300), (150, 3)));
    }

    #[test]
    fn opens_away_from_the_screen_edge() {
        assert!(!far(100, 34, 400, (0, 1040), 16));
        assert!(far(900, 34, 400, (0, 1040), 16));
        assert!(!far(900, 34, 100, (0, 1040), 16));
        assert!(!far(500, 34, 900, (0, 1040), 16));
        assert!(far(600, 34, 900, (0, 1040), 16));
        assert_eq!((Side::pick(false, true), Side::pick(true, true), Side::pick(true, false)), (Side::Bottom, Side::Right, Side::Left));
        assert!(Side::Bottom.flipped() && Side::Left.flipped() && !Side::Right.flipped() && Side::Right.far());
    }
}
