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
    (w / cell.max(1)).max(1)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flow {
    pub w: i32,
    pub cell: i32,
    pub cw: i32,
    pub n: usize,
    pub align: Align,
}

impl Flow {
    fn row_len(&self, row: i32) -> i32 {
        let c = cols(self.w, self.cw);
        (self.n as i32 - row * c).clamp(0, c)
    }

    fn start(&self, row: i32) -> i32 {
        let free = self.w - self.row_len(row) * self.cw;
        (match self.align {
            Align::Left => 0,
            Align::Center => free / 2,
            Align::Right => free,
        })
        .max(0)
    }

    pub fn origin(&self, i: usize) -> (i32, i32) {
        let (c, i) = (cols(self.w, self.cw), i as i32);
        (self.start(i / c) + i % c * self.cw, i / c * self.cell)
    }

    pub fn index_at(&self, (x, y): (i32, i32)) -> Option<usize> {
        let row = (y >= 0).then_some(y / self.cell)?;
        let x = x - self.start(row);
        let col = (x >= 0).then_some(x / self.cw.max(1)).filter(|&c| c < self.row_len(row))?;
        Some((row * cols(self.w, self.cw) + col) as usize)
    }

    pub fn in_rect(&self, (x0, y0, x1, y1): (i32, i32, i32, i32)) -> Vec<usize> {
        let (l, r, t, b) = (x0.min(x1), x0.max(x1), y0.min(y1), y0.max(y1));
        (0..self.n)
            .filter(|&i| {
                let (x, y) = self.origin(i);
                x < r && x + self.cw > l && y < b && y + self.cell > t
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
#[path = "grid_tests.rs"]
mod tests;
