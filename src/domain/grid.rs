use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

impl Align {
    pub const ALL: [Align; 3] = [Align::Left, Align::Center, Align::Right];

    pub fn mirrored(self) -> Align {
        match self {
            Align::Left => Align::Right,
            Align::Center => Align::Center,
            Align::Right => Align::Left,
        }
    }
}

pub fn cols(w: i32, cell: i32) -> i32 {
    (w / cell).max(1)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flow {
    pub w: i32,
    pub cell: i32,
    pub n: usize,
    pub align: Align,
}

impl Flow {
    fn row_len(&self, row: i32) -> i32 {
        let c = cols(self.w, self.cell);
        (self.n as i32 - row * c).clamp(0, c)
    }

    fn start(&self, row: i32) -> i32 {
        let free = self.w - self.row_len(row) * self.cell;
        (match self.align {
            Align::Left => 0,
            Align::Center => free / 2,
            Align::Right => free,
        })
        .max(0)
    }

    pub fn origin(&self, i: usize) -> (i32, i32) {
        let (c, i) = (cols(self.w, self.cell), i as i32);
        (self.start(i / c) + i % c * self.cell, i / c * self.cell)
    }

    pub fn index_at(&self, (x, y): (i32, i32)) -> Option<usize> {
        let row = (y >= 0).then_some(y / self.cell)?;
        let x = x - self.start(row);
        let col = (x >= 0).then_some(x / self.cell).filter(|&c| c < self.row_len(row))?;
        Some((row * cols(self.w, self.cell) + col) as usize)
    }

    pub fn in_rect(&self, (x0, y0, x1, y1): (i32, i32, i32, i32)) -> Vec<usize> {
        let (l, r, t, b) = (x0.min(x1), x0.max(x1), y0.min(y1), y0.max(y1));
        let cell = self.cell;
        (0..self.n)
            .filter(|&i| {
                let (x, y) = self.origin(i);
                x < r && x + cell > l && y < b && y + cell > t
            })
            .collect()
    }
}

pub fn max_scroll(n: usize, w: i32, h: i32, cell: i32) -> i32 {
    let c = cols(w, cell);
    ((n as i32 + c - 1) / c * cell - h).max(0)
}

pub fn fit(n: usize, w: i32, cell: i32, chrome: i32, (min, max): (i32, i32)) -> i32 {
    let rows = (n as i32 + cols(w, cell) - 1) / cols(w, cell);
    (chrome + rows.max(1) * cell).clamp(min, max.max(min))
}

pub fn fit_cols(n: usize, len: i32, cell: i32, chrome: i32, (min, max): (i32, i32)) -> i32 {
    let rows = (len / cell.max(1)).max(1);
    let cols = (n as i32 + rows - 1) / rows;
    (chrome + cols.max(1) * cell).clamp(min, max.max(min))
}

pub fn split(w: i32, n: usize, x: i32) -> Option<usize> {
    let n = n.max(1) as i32;
    (0..w).contains(&x).then(|| (x * n / w.max(1)) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_layout() {
        let flow = |n, align| Flow { w: 250, cell: 80, n, align };
        assert_eq!(cols(250, 80), 3);
        assert_eq!((flow(5, Align::Center).origin(1), flow(5, Align::Center).origin(4)), ((85, 0), (125, 80)));
        assert_eq!(
            (flow(5, Align::Left).origin(4), flow(5, Align::Right).origin(3), flow(5, Align::Right).origin(2)),
            ((80, 80), (90, 80), (170, 0))
        );
        assert_eq!(flow(5, Align::Center).index_at((125, 80)), Some(4));
        assert_eq!(flow(5, Align::Center).index_at((20, 85)), None);
        assert_eq!(flow(5, Align::Center).index_at((2, 10)), None);
        assert_eq!(flow(5, Align::Right).index_at((95, 85)), Some(3));
        assert_eq!(flow(5, Align::Left).index_at((10, 10)), Some(0));
        assert_eq!(flow(5, Align::Left).index_at((10, -5)), None);
        assert_eq!((Align::Left.mirrored(), Align::Center.mirrored()), (Align::Right, Align::Center));
        assert_eq!(max_scroll(7, 250, 100, 80), 140);
        assert_eq!(max_scroll(1, 250, 100, 80), 0);
    }

    #[test]
    fn auto_height() {
        assert_eq!(fit(5, 250, 80, 50, (80, 1000)), 50 + 2 * 80);
        assert_eq!(fit(0, 250, 80, 50, (80, 1000)), 50 + 80);
        assert_eq!(fit(6, 250, 80, 50, (80, 1000)), 50 + 2 * 80);
        assert_eq!(fit(40, 250, 80, 50, (80, 500)), 500);
        assert_eq!(fit(1, 250, 10, 5, (80, 500)), 80);
    }

    #[test]
    fn auto_width() {
        assert_eq!(fit_cols(5, 250, 80, 40, (60, 1000)), 40 + 2 * 80);
        assert_eq!(fit_cols(0, 250, 80, 40, (60, 1000)), 40 + 80);
        assert_eq!(fit_cols(30, 250, 80, 40, (60, 500)), 500);
        assert_eq!(fit_cols(3, 10, 80, 40, (60, 1000)), 40 + 3 * 80);
    }

    #[test]
    fn band_selection() {
        let flow = |n| Flow { w: 250, cell: 80, n, align: Align::Center };
        assert_eq!(flow(5).in_rect((90, 5, 100, 90)), vec![1, 3]);
        assert_eq!(flow(5).in_rect((100, 90, 90, 5)), vec![1, 3]);
        assert_eq!(flow(5).in_rect((0, 0, 4, 4)), Vec::<usize>::new());
        assert_eq!(flow(3).in_rect((0, 0, 250, 300)), vec![0, 1, 2]);
    }

    #[test]
    fn tab_split() {
        assert_eq!(split(300, 3, 0), Some(0));
        assert_eq!(split(300, 3, 150), Some(1));
        assert_eq!(split(300, 3, 299), Some(2));
        assert_eq!(split(300, 3, 300), None);
        assert_eq!(split(300, 1, 10), Some(0));
    }
}
