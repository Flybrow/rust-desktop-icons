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
    pub head: i32,
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
        (if self.side.vertical() { y1 - y0 } else { y1 - y0 - self.gap }) - self.head
    }

    pub fn to_window(self, (x, y): (i32, i32)) -> (i32, i32) {
        let ((x0, y0, x1, y1), g, y) = (self.body(), self.gap, y + self.head);
        match self.side {
            Side::Top => (x0 + x, y0 + g + y),
            Side::Bottom => (x0 + x, y1 - g - y),
            Side::Left => (x0 + g + x, y0 + y),
            Side::Right => (x1 - g - x, y0 + y),
        }
    }

    pub fn to_content(self, (px, py): (i32, i32)) -> (i32, i32) {
        let ((x0, y0, x1, y1), g, k) = (self.body(), self.gap, self.head);
        match self.side {
            Side::Top => (px - x0, py - y0 - g - k),
            Side::Bottom => (px - x0, y1 - g - k - py),
            Side::Left => (px - x0 - g, py - y0 - k),
            Side::Right => (x1 - g - px, py - y0 - k),
        }
    }

    pub fn head_rect(&self) -> Rect {
        self.rect((0, -self.head, self.grid_w(), 0))
    }

    pub fn rows(&self) -> Rect {
        let ((x0, y0, x1, y1), g, k) = (self.body(), self.gap, self.head);
        match self.side {
            _ if k == 0 => (x0, y0, x1, y1),
            Side::Top => (x0, y0 + g + k, x1, y1),
            Side::Bottom => (x0, y0, x1, y1 - g - k),
            _ => (x0, y0 + k, x1, y1),
        }
    }

    pub fn rect(&self, (x0, y0, x1, y1): Rect) -> Rect {
        let ((ax, ay), (bx, by)) = (self.to_window((x0, y0)), self.to_window((x1, y1)));
        (ax.min(bx), ay.min(by), ax.max(bx), ay.max(by))
    }

    pub fn cell(&self, (x, y): (i32, i32), (w, h): (i32, i32)) -> (i32, i32) {
        let (l, t, _, _) = self.rect((x, y, x + w, y + h));
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
#[path = "side_tests.rs"]
mod tests;
