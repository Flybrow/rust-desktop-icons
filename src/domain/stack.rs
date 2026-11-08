use super::pins::{Wall, corner_slot, free_slots};

pub const AWAY: i32 = -20_000;

pub type Point = (i32, i32);

pub fn away((x, _): Point) -> bool {
    x <= AWAY / 2
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Icon {
    pub pos: Point,
    pub kept: bool,
}

#[derive(Debug, Default, PartialEq)]
pub struct Plan {
    pub stack: Point,
    pub hide: Vec<usize>,
    pub stacked: Vec<usize>,
    pub show: Vec<(usize, Point)>,
    pub any_kept: bool,
}

pub fn plan(icons: &[Icon], prev: Option<Point>, (cw, ch): Point, area: (i32, i32, i32, i32), walls: &[Wall]) -> Plan {
    let others: Vec<Point> = icons.iter().filter(|i| !away(i.pos) && !i.kept).map(|i| i.pos).collect();
    let inside = |(x, y): Point| x >= area.0 && y >= area.1 && x + cw <= area.0 + area.2 && y + ch <= area.1 + area.3;
    let near = |(x, y): Point, (ox, oy): Point| (ox - x).abs() < cw && (oy - y).abs() < ch;
    let free = |p: &Point| inside(*p) && !others.iter().any(|o| near(*p, *o));
    let stack = prev.filter(free).unwrap_or_else(|| corner_slot(&others, (cw, ch), area));
    let idx = |f: &dyn Fn(&Icon) -> bool| icons.iter().enumerate().filter(|(_, i)| f(i)).map(|(k, _)| k).collect::<Vec<_>>();
    let hide = idx(&|i| i.kept && i.pos != stack);
    let stacked = idx(&|i| i.pos == stack);
    let shown = idx(&|i| !i.kept && (away(i.pos) || i.pos == stack));
    let taken: Vec<Point> = others.iter().copied().chain([stack]).collect();
    let slots = if shown.is_empty() { vec![] } else { free_slots(&taken, walls, (cw, ch), area, shown.len()) };
    Plan { stack, hide, stacked, show: shown.into_iter().zip(slots).collect(), any_kept: icons.iter().any(|i| i.kept) }
}

pub fn row((x, y): Point, (cw, ch): Point, n: usize) -> Vec<Point> {
    (0..n as i32).map(|i| (x - cw / 2 + i * cw, y - ch / 2)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CELL: Point = (100, 100);
    const AREA: (i32, i32, i32, i32) = (0, 0, 1000, 500);

    fn icon(pos: Point, kept: bool) -> Icon {
        Icon { pos, kept }
    }

    #[test]
    fn kept_icons_join_the_corner_stack() {
        let icons = [icon((0, 0), true), icon((100, 0), false), icon((900, 400), true)];
        let p = plan(&icons, None, CELL, AREA, &[]);
        assert_eq!((p.stack, p.hide, p.stacked, p.any_kept), ((900, 400), vec![0], vec![2], true));
        assert!(p.show.is_empty());
    }

    #[test]
    fn loose_icons_leave_the_stack_and_avoid_fences() {
        let icons = [icon((900, 400), false), icon((AWAY, 0), false), icon((900, 300), true)];
        let p = plan(&icons, Some((900, 400)), CELL, AREA, &[(0, 0, 150, 500)]);
        assert_eq!(p.stack, (900, 300));
        assert_eq!(p.show, vec![(1, (200, 0))]);
        assert!(p.hide.is_empty());
        let p = plan(&[icon((500, 200), false)], Some((900, 400)), CELL, AREA, &[]);
        assert_eq!((p.stack, p.any_kept, p.show.len()), ((900, 400), false, 0));
    }

    #[test]
    fn dropped_items_form_a_row() {
        assert_eq!(row((500, 300), CELL, 3), vec![(450, 250), (550, 250), (650, 250)]);
    }

    #[test]
    fn only_far_away_icons_count_as_parked() {
        assert!(away((AWAY, 0)) && away((AWAY / 2, 5)) && !away((AWAY / 2 + 1, 0)) && !away((-5, 0)) && !away((0, 0)));
    }

    #[test]
    fn a_previous_stack_cell_is_kept_only_while_free_and_inside() {
        let icons = [icon((300, 200), true), icon((500, 200), false)];
        let keep = plan(&icons, Some((300, 200)), CELL, AREA, &[]);
        assert_eq!(keep.stack, (300, 200));
        for prev in [(450, 150), (550, 250), (-50, 0), (950, 0), (0, 450), (0, -1), (901, 0), (0, 401)] {
            assert_eq!(plan(&icons, Some(prev), CELL, AREA, &[]).stack, (900, 400), "{prev:?}");
        }
        for prev in [(400, 200), (600, 200), (500, 100), (500, 300), (0, 0), (900, 400), (0, 400), (900, 0)] {
            assert_eq!(plan(&icons, Some(prev), CELL, AREA, &[]).stack, prev, "{prev:?}");
        }
    }
}
