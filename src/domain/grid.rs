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

fn rows(n: usize, w: i32, cell: i32) -> i32 {
    (n as i32 + cols(w, cell) - 1) / cols(w, cell)
}

pub fn last_row(n: usize, w: i32, cell: i32) -> std::ops::Range<usize> {
    ((rows(n, w, cell) - 1).max(0) * cols(w, cell)) as usize..n
}

pub fn extent(n: usize, w: i32, cell: i32, tail: i32) -> i32 {
    (rows(n, w, cell).max(1) - 1) * cell + tail
}

pub fn max_scroll(n: usize, w: i32, h: i32, cell: i32, tail: i32) -> i32 {
    if n == 0 { 0 } else { (extent(n, w, cell, tail) - h).max(0) }
}

pub fn fit(n: usize, w: i32, (cell, tail): (i32, i32), chrome: i32, (min, max): (i32, i32)) -> i32 {
    (chrome + extent(n, w, cell, tail)).clamp(min, max.max(min))
}

pub fn fit_cols(n: usize, len: i32, cell: i32, chrome: i32, (min, max): (i32, i32)) -> i32 {
    let rows = (len / cell.max(1)).max(1);
    let cols = (n as i32 + rows - 1) / rows;
    (chrome + cols.max(1) * cell).clamp(min, max.max(min))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Move {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Page(i32),
}

pub fn step(n: usize, cols: i32, at: Option<usize>, m: Move, (flip_x, flip_y): (bool, bool)) -> Option<usize> {
    let last = n.checked_sub(1)? as i32;
    let Some(i) = at.map(|i| i.min(last as usize) as i32) else { return Some(0) };
    let cols = cols.max(1);
    let m = match m {
        Move::Left if flip_x => Move::Right,
        Move::Right if flip_x => Move::Left,
        Move::Up if flip_y => Move::Down,
        Move::Down if flip_y => Move::Up,
        Move::Page(r) if flip_y => Move::Page(-r),
        m => m,
    };
    let to = match m {
        Move::Left => i - 1,
        Move::Right => i + 1,
        Move::Up if i >= cols => i - cols,
        Move::Up => i,
        Move::Down if i + cols <= last => i + cols,
        Move::Down if i / cols < last / cols => last,
        Move::Down => i,
        Move::Home => 0,
        Move::End => last,
        Move::Page(r) => i + r * cols,
    };
    Some(to.clamp(0, last) as usize)
}

pub fn row_span(i: usize, cols: i32, cell: i32) -> (i32, i32) {
    let top = i as i32 / cols.max(1) * cell;
    (top, top + cell)
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
        assert_eq!(max_scroll(7, 250, 100, 80, 80), 140);
        assert_eq!((max_scroll(7, 250, 100, 80, 50), max_scroll(1, 250, 100, 80, 80), max_scroll(0, 250, 10, 80, 80)), (110, 0, 0));
        assert_eq!((last_row(7, 250, 80), last_row(6, 250, 80), last_row(2, 250, 80), last_row(0, 250, 80)), (6..7, 3..6, 0..2, 0..0));
    }

    #[test]
    fn auto_height() {
        assert_eq!(fit(5, 250, (80, 80), 50, (80, 1000)), 50 + 2 * 80);
        assert_eq!(fit(5, 250, (80, 62), 50, (80, 1000)), 50 + 80 + 62);
        assert_eq!(fit(0, 250, (80, 62), 50, (80, 1000)), 50 + 62);
        assert_eq!(fit(6, 250, (80, 80), 50, (80, 1000)), 50 + 2 * 80);
        assert_eq!(fit(40, 250, (80, 80), 50, (80, 500)), 500);
        assert_eq!(fit(1, 250, (10, 10), 5, (80, 500)), 80);
    }

    #[test]
    fn auto_width() {
        assert_eq!(fit_cols(5, 250, 80, 40, (60, 1000)), 40 + 2 * 80);
        assert_eq!(fit_cols(0, 250, 80, 40, (60, 1000)), 40 + 80);
        assert_eq!(fit_cols(30, 250, 80, 40, (60, 500)), 500);
        assert_eq!(fit_cols(3, 10, 80, 40, (60, 1000)), 40 + 3 * 80);
    }

    #[test]
    fn keyboard_steps() {
        let go = |at, m| step(7, 3, at, m, (false, false));
        assert_eq!((go(None, Move::Down), go(Some(4), Move::Left), go(Some(4), Move::Right)), (Some(0), Some(3), Some(5)));
        assert_eq!((go(Some(0), Move::Left), go(Some(6), Move::Right), go(Some(1), Move::Up)), (Some(0), Some(6), Some(1)));
        assert_eq!((go(Some(1), Move::Down), go(Some(4), Move::Down), go(Some(6), Move::Down)), (Some(4), Some(6), Some(6)));
        assert_eq!(
            (go(Some(5), Move::Home), go(Some(0), Move::End), go(Some(0), Move::Page(2)), go(Some(6), Move::Page(-1))),
            (Some(0), Some(6), Some(6), Some(3))
        );
        assert_eq!((step(7, 3, Some(4), Move::Left, (true, false)), step(7, 3, Some(4), Move::Up, (false, true))), (Some(5), Some(6)));
        assert_eq!((step(0, 3, None, Move::Down, (false, false)), row_span(4, 3, 80)), (None, (80, 160)));
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

    #[test]
    fn every_arrow_moves_one_cell_on_any_bar_side() {
        let go = |at, m, flip| step(8, 3, Some(at), m, flip);
        let plain = (false, false);
        assert_eq!(
            (go(4, Move::Up, plain), go(3, Move::Up, plain), go(4, Move::Down, plain), go(5, Move::Down, plain)),
            (Some(1), Some(0), Some(7), Some(7))
        );
        assert_eq!((go(1, Move::Down, plain), go(2, Move::Down, plain), go(6, Move::Down, plain)), (Some(4), Some(5), Some(6)));
        assert_eq!((go(4, Move::Right, (true, false)), go(4, Move::Left, (true, false))), (Some(3), Some(5)));
        assert_eq!((go(4, Move::Down, (false, true)), go(4, Move::Up, (false, true))), (Some(1), Some(7)));
        assert_eq!((go(4, Move::Page(1), (false, true)), go(4, Move::Page(1), plain)), (Some(1), Some(7)));
        assert_eq!((go(4, Move::Right, (false, true)), go(4, Move::Up, (true, false))), (Some(5), Some(1)));
        assert_eq!((step(5, 2, Some(2), Move::Down, plain), step(5, 2, Some(1), Move::Down, plain)), (Some(4), Some(3)));
        assert_eq!((row_span(5, 3, 80), row_span(2, 3, 80)), ((80, 160), (0, 80)));
    }

    #[test]
    fn hits_and_bands_stop_at_cell_edges() {
        let flow = |n| Flow { w: 250, cell: 80, n, align: Align::Left };
        assert_eq!((flow(5).index_at((170, 90)), flow(5).index_at((90, 90))), (None, Some(4)));
        assert_eq!((flow(3).in_rect((0, 0, 80, 80)), flow(5).in_rect((0, 0, 250, 80))), (vec![0], vec![0, 1, 2]));
        assert!(flow(5).in_rect((160, 160, 300, 300)).is_empty() && flow(5).in_rect((0, 160, 250, 300)).is_empty());
        assert_eq!(step(8, 3, Some(3), Move::Down, (false, false)), Some(6));
    }
}
