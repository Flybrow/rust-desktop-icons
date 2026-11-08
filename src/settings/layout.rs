use super::rows::{Ctl, PAGES, Row};

pub const ITEM: i32 = 40;
const HEADER: i32 = 46;
const ROW: i32 = 44;
const NOTE: i32 = 84;
const DESCRIBED: i32 = 66;
pub const RESET_LINE: i32 = 22;

pub fn height(r: &Row) -> i32 {
    let base = match r.ctl {
        Ctl::Header => HEADER,
        Ctl::Note => NOTE,
        _ if !r.desc.is_empty() => DESCRIBED,
        _ => ROW,
    };
    base + if r.reset.is_some() { RESET_LINE } else { 0 }
}

pub fn on_reset(r: &Row, offset: i32) -> bool {
    r.reset.is_some() && offset >= height(r) - RESET_LINE
}

pub fn layout(rows: &[Row]) -> Vec<(i32, i32)> {
    rows.iter().scan(0, |y, r| Some((std::mem::replace(y, *y + height(r)), height(r)))).collect()
}

pub fn cards(rows: &[Row]) -> Vec<(usize, usize)> {
    let mut out = vec![];
    for (i, r) in rows.iter().enumerate() {
        match (r.ctl == Ctl::Header, out.last_mut()) {
            (true, _) => {}
            (false, Some((_, end))) if *end + 1 == i => *end = i,
            _ => out.push((i, i)),
        }
    }
    out
}

pub fn total(rows: &[Row]) -> i32 {
    rows.iter().map(height).sum()
}

pub const SECOND_LINE: i32 = 31;

pub fn hit(rows: &[Row], y: i32) -> Option<(usize, i32)> {
    let i = layout(rows).iter().position(|&(top, h)| (top..top + h).contains(&y)).filter(|&i| rows[i].act.is_some())?;
    Some((i, y - layout(rows)[i].0))
}

pub fn page_at(y: i32) -> Option<usize> {
    (y >= 0).then_some((y / ITEM) as usize).filter(|&i| i < PAGES.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::rows::{Act, header, note, row};

    #[test]
    fn row_layout() {
        let rows = vec![header("A"), row("b", Ctl::Button, Act::Check, 'x'), row("c", Ctl::Toggle(true), Act::AutoSort, 'x'), note("n")];
        assert_eq!(layout(&rows), vec![(0, HEADER), (HEADER, ROW), (HEADER + ROW, ROW), (HEADER + 2 * ROW, NOTE)]);
        assert_eq!(total(&rows), HEADER + 2 * ROW + NOTE);
        assert_eq!(hit(&rows, 5).map(|h| h.0), None);
        assert_eq!(hit(&rows, HEADER + 1).map(|h| h.0), Some(1));
        assert_eq!(hit(&rows, HEADER + ROW + ROW - 1).map(|h| h.0), Some(2));
        assert_eq!(hit(&rows, HEADER + 2 * ROW + 1).map(|h| h.0), None);
        assert_eq!(hit(&rows, total(&rows)).map(|h| h.0), None);
    }

    #[test]
    fn described_rows_have_a_second_line() {
        let rows = vec![header("A"), row("b", Ctl::Value("v".into()), Act::Check, 'x').about("why")];
        assert_eq!(hit(&rows, HEADER + 5), Some((1, 5)));
        assert_eq!(layout(&rows)[1].1, DESCRIBED);
        let reset = Row { reset: Some(Act::Check), ..rows[1].clone() };
        assert_eq!(height(&reset), DESCRIBED + RESET_LINE);
        assert!(on_reset(&reset, DESCRIBED + 2) && !on_reset(&reset, SECOND_LINE + 2) && !on_reset(&rows[1], DESCRIBED + 2));
    }

    #[test]
    fn card_groups() {
        let r = |c| row("r", c, Act::Check, 'x');
        let rows = vec![header("A"), r(Ctl::Button), r(Ctl::Button), header("B"), r(Ctl::Toggle(false))];
        assert_eq!(cards(&rows), vec![(1, 2), (4, 4)]);
    }

    #[test]
    fn sidebar_pages() {
        assert_eq!((page_at(-1), page_at(0), page_at(ITEM * 2 + 3), page_at(ITEM * PAGES.len() as i32)), (None, Some(0), Some(2), None));
    }
}
