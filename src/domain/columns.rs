use super::order::Sort;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Icons,
    List,
    Details,
    Tiles,
    Content,
}

impl Mode {
    pub const ALL: [Mode; 5] = [Mode::Icons, Mode::List, Mode::Details, Mode::Tiles, Mode::Content];

    pub fn rich(self) -> bool {
        matches!(self, Mode::Details | Mode::Tiles | Mode::Content)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Col {
    Name,
    Modified,
    Created,
    Type,
    Size,
}

impl Col {
    pub const ALL: [Col; 5] = [Col::Name, Col::Modified, Col::Created, Col::Type, Col::Size];

    pub fn sort(self) -> Sort {
        match self {
            Col::Name => Sort::Name,
            Col::Modified => Sort::Date,
            Col::Created => Sort::Created,
            Col::Type => Sort::Type,
            Col::Size => Sort::Size,
        }
    }

    fn width(self) -> i32 {
        match self {
            Col::Name => NAME_MIN,
            Col::Modified | Col::Created => 130,
            Col::Type => 120,
            Col::Size => 76,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Column {
    pub col: Col,
    pub w: i32,
}

pub const MIN: i32 = 40;
pub const NAME_MIN: i32 = 80;

pub type Span = (Col, i32, i32);

pub fn defaults() -> Vec<Column> {
    [Col::Modified, Col::Type, Col::Size].map(|col| Column { col, w: col.width() }).to_vec()
}

pub fn sanitized(cols: Vec<Column>) -> Vec<Column> {
    let mut out: Vec<Column> = vec![];
    for c in cols.into_iter().filter(|c| c.col != Col::Name) {
        if !out.iter().any(|o| o.col == c.col) {
            out.push(Column { w: c.w.clamp(MIN, 2000), ..c });
        }
    }
    out
}

pub fn spans(cols: &[Column], width: i32, s: impl Fn(i32) -> i32) -> Vec<Span> {
    let fixed: i32 = cols.iter().map(|c| s(c.w)).sum();
    let mut x = (width - fixed).max(s(NAME_MIN));
    let mut out = vec![(Col::Name, 0, x)];
    for c in cols {
        out.push((c.col, x, x + s(c.w)));
        x += s(c.w);
    }
    out
}

pub fn at(spans: &[Span], x: i32) -> Option<Col> {
    spans.iter().find(|&&(_, a, b)| (a..b).contains(&x)).map(|s| s.0)
}

pub fn edge(spans: &[Span], x: i32, tol: i32) -> Option<usize> {
    spans.iter().position(|&(_, _, b)| (x - b).abs() <= tol)
}

pub fn resize(cols: &[Column], spans: &[Span], edge: usize, x: i32, unscale: impl Fn(i32) -> i32) -> Vec<Column> {
    let mut out = cols.to_vec();
    let Some(&(_, a, b)) = spans.get(edge) else { return out };
    let dx = unscale(x - b);
    match (edge.checked_sub(1), out.get(edge).map(|c| c.w)) {
        (None, Some(next)) => out[0].w = (next - dx).max(MIN),
        (Some(i), Some(next)) => {
            let total = out[i].w + next;
            out[i].w = (out[i].w + dx).clamp(MIN, total - MIN);
            out[edge].w = total - out[i].w;
        }
        (Some(i), None) => out[i].w = unscale(x - a).max(MIN),
        _ => {}
    }
    out
}

pub fn toggle(cols: &[Column], col: Col) -> Vec<Column> {
    if col == Col::Name {
        return cols.to_vec();
    }
    if cols.iter().any(|c| c.col == col) {
        return cols.iter().copied().filter(|c| c.col != col).collect();
    }
    let rank = |c: Col| Col::ALL.iter().position(|&k| k == c);
    let mut out = cols.to_vec();
    let at = out.iter().position(|c| rank(c.col) > rank(col)).unwrap_or(out.len());
    out.insert(at, Column { col, w: col.width() });
    out
}

pub fn click(current: (Sort, bool), col: Col) -> (Sort, bool) {
    match current {
        (sort, desc) if sort == col.sort() => (sort, !desc),
        _ => (col.sort(), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(col: Col, w: i32) -> Column {
        Column { col, w }
    }

    #[test]
    fn the_name_column_takes_the_rest() {
        let s = |v: i32| v * 2;
        let sp = spans(&defaults(), 1000, s);
        assert_eq!(sp, vec![(Col::Name, 0, 348), (Col::Modified, 348, 608), (Col::Type, 608, 848), (Col::Size, 848, 1000)]);
        assert_eq!(spans(&defaults(), 300, s)[0], (Col::Name, 0, 160));
        assert_eq!(spans(&[], 500, s), vec![(Col::Name, 0, 500)]);
        assert_eq!((at(&sp, 0), at(&sp, 700), at(&sp, 1000)), (Some(Col::Name), Some(Col::Type), None));
        assert_eq!((edge(&sp, 350, 4), edge(&sp, 999, 4), edge(&sp, 500, 4)), (Some(0), Some(3), None));
    }

    #[test]
    fn dragging_an_edge_moves_the_border_between_its_two_columns() {
        let (cols, half) = (defaults(), |v: i32| v / 2);
        let sp = spans(&cols, 1000, |v| v * 2);
        assert_eq!(resize(&cols, &sp, 1, 608 + 40, half)[..2], [c(Col::Modified, 150), c(Col::Type, 100)]);
        assert_eq!(resize(&cols, &sp, 1, 608 - 40, half)[..2], [c(Col::Modified, 110), c(Col::Type, 140)]);
        assert_eq!(resize(&cols, &sp, 1, 2000, half)[..2], [c(Col::Modified, 250 - MIN), c(Col::Type, MIN)]);
        assert_eq!(resize(&cols, &sp, 1, 0, half)[..2], [c(Col::Modified, MIN), c(Col::Type, 250 - MIN)]);
        assert_eq!(resize(&cols, &sp, 1, 700, half)[2], cols[2]);
        assert_eq!(resize(&cols, &sp, 3, 1000 - 20, half)[2], c(Col::Size, 66));
        assert_eq!(resize(&cols, &sp, 0, 348 + 40, half)[0], c(Col::Modified, 110));
        assert_eq!(resize(&cols, &sp, 0, 348 - 40, half)[0], c(Col::Modified, 150));
        assert_eq!(resize(&[], &spans(&[], 500, |v| v), 0, 300, half), vec![]);
        assert_eq!(resize(&cols, &sp, 9, 0, half), cols);
    }

    #[test]
    fn columns_can_be_shown_and_hidden_in_a_fixed_order() {
        let cols = defaults();
        let no_type = toggle(&cols, Col::Type);
        assert_eq!(no_type, vec![c(Col::Modified, 130), c(Col::Size, 76)]);
        assert_eq!(toggle(&no_type, Col::Type), cols);
        assert_eq!(toggle(&cols, Col::Created)[1], c(Col::Created, 130));
        assert_eq!(toggle(&cols, Col::Name), cols);
        assert_eq!(toggle(&[], Col::Size), vec![c(Col::Size, 76)]);
        let messy = vec![c(Col::Name, 9), c(Col::Size, 5), c(Col::Size, 300), c(Col::Type, 9999)];
        assert_eq!(sanitized(messy), vec![c(Col::Size, MIN), c(Col::Type, 2000)]);
    }

    #[test]
    fn a_header_click_sorts_then_reverses() {
        assert_eq!(click((Sort::Name, false), Col::Size), (Sort::Size, false));
        assert_eq!(click((Sort::Size, false), Col::Size), (Sort::Size, true));
        assert_eq!(click((Sort::Size, true), Col::Size), (Sort::Size, false));
        assert_eq!(click((Sort::Custom, true), Col::Created), (Sort::Created, false));
        assert_eq!(Mode::ALL.iter().filter(|m| m.rich()).count(), 3);
        assert!(Mode::Tiles.rich() && !Mode::List.rich() && !Mode::Icons.rich());
    }
}
