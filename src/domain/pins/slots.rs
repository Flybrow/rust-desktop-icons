pub type Wall = (i32, i32, i32, i32);

pub fn covered(walls: &[Wall], (x, y): (i32, i32), (cw, ch): (i32, i32)) -> bool {
    walls.iter().any(|&(l, t, r, b)| x < r && x + cw > l && y < b && y + ch > t)
}

fn columns(cell: (i32, i32), (ox, oy, _, h): (i32, i32, i32, i32)) -> impl Iterator<Item = (i32, i32)> {
    let (cw, ch) = (cell.0.max(1), cell.1.max(1));
    let rows = (h / ch).max(1);
    (0..).map(move |i| (ox + i / rows * cw, oy + i % rows * ch))
}

fn busy(taken: &[(i32, i32)], walls: &[Wall], (cw, ch): (i32, i32)) -> impl Fn(&(i32, i32)) -> bool {
    move |&(x, y)| taken.iter().any(|&(tx, ty)| (tx - x).abs() < cw && (ty - y).abs() < ch) || covered(walls, (x, y), (cw, ch))
}

pub fn free_slots(taken: &[(i32, i32)], walls: &[Wall], cell: (i32, i32), area: (i32, i32, i32, i32), n: usize) -> Vec<(i32, i32)> {
    let (cw, ch) = (cell.0.max(1), cell.1.max(1));
    let used = busy(taken, walls, (cw, ch));
    let fits = |&(x, y): &(i32, i32)| x + cw <= area.0 + area.2 && y + ch <= area.1 + area.3;
    let cells = (area.2 / cw).max(1) as usize * (area.3 / ch).max(1) as usize;
    columns(cell, area).take(cells).filter(fits).filter(|p| !used(p)).take(n).collect()
}

pub fn clear_spot(taken: &[(i32, i32)], walls: &[Wall], cell: (i32, i32), area: (i32, i32, i32, i32)) -> Option<(i32, i32)> {
    free_slots(taken, walls, cell, area, 1).pop()
}

pub fn corner_slot(taken: &[(i32, i32)], cell: (i32, i32), (ox, oy, w, h): (i32, i32, i32, i32)) -> (i32, i32) {
    let (cw, ch) = (cell.0.max(1), cell.1.max(1));
    let rows = (h / ch).max(1);
    let used = busy(taken, &[], (cw, ch));
    (0..rows * (w / cw).max(1))
        .map(|i| (ox + w - (i / rows + 1) * cw, oy + h - (i % rows + 1) * ch))
        .find(|p| !used(p))
        .unwrap_or((ox + w - cw, oy + h - ch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_fill_columns_and_skip_taken() {
        assert_eq!(free_slots(&[], &[], (100, 100), (0, 0, 1000, 250), 3), vec![(0, 0), (0, 100), (100, 0)]);
        assert_eq!(free_slots(&[(0, 5), (0, 100)], &[], (100, 100), (0, 0, 1000, 200), 2), vec![(100, 0), (100, 100)]);
        assert!(free_slots(&[], &[], (0, 0), (0, 0, 0, 0), 2).is_empty());
        assert_eq!(free_slots(&[(1920, 0)], &[], (100, 100), (1920, 0, 1000, 200), 2), vec![(1920, 100), (2020, 0)]);
        assert_eq!(free_slots(&[], &[(0, 0, 150, 120)], (100, 100), (0, 0, 1000, 300), 2), vec![(0, 200), (100, 200)]);
    }

    #[test]
    fn slots_never_leave_the_area() {
        let area = (50, 20, 250, 200);
        let all = free_slots(&[], &[], (100, 100), area, 10);
        assert_eq!(all, vec![(50, 20), (50, 120), (150, 20), (150, 120)]);
        assert!(all.iter().all(|&(x, y)| x >= 50 && y >= 20 && x + 100 <= 300 && y + 100 <= 220));
        assert_eq!(free_slots(&[(50, 20), (150, 120)], &[], (100, 100), area, 10), vec![(50, 120), (150, 20)]);
        assert!(free_slots(&[], &[(0, 0, 400, 400)], (100, 100), area, 3).is_empty());
        assert!(free_slots(&[], &[], (100, 100), (0, 0, 50, 300), 3).is_empty());
    }

    #[test]
    fn new_items_avoid_fences() {
        let walls = [(0, 0, 250, 1000)];
        assert!(covered(&walls, (240, 10), (100, 100)) && !covered(&walls, (250, 10), (100, 100)));
        assert_eq!(clear_spot(&[(300, 0)], &walls, (100, 100), (0, 0, 1000, 300)), Some((300, 100)));
        assert_eq!(clear_spot(&[], &[(0, 0, 1000, 300)], (100, 100), (0, 0, 1000, 300)), None);
        assert_eq!(clear_spot(&[(0, 0)], &[], (100, 100), (0, 0, 150, 150)), None);
    }

    #[test]
    fn hidden_stack_sits_in_the_bottom_right_free_cell() {
        let area = (1920, 0, 1000, 1000);
        assert_eq!(corner_slot(&[], (100, 100), area), (2820, 900));
        assert_eq!(corner_slot(&[(2830, 905)], (100, 100), area), (2820, 800));
        let full: Vec<(i32, i32)> = (0..10).map(|r| (2820, r * 100)).collect();
        assert_eq!(corner_slot(&full, (100, 100), area), (2720, 900));
        assert_eq!(corner_slot(&[], (0, 0), (0, 0, 0, 0)), (-1, -1));
    }

    #[test]
    fn cells_touching_a_wall_edge_are_free() {
        let wall = [(100, 100, 200, 200)];
        assert!(!covered(&wall, (0, 100), (100, 100)) && !covered(&wall, (200, 100), (100, 100)));
        assert!(!covered(&wall, (100, 0), (100, 100)) && !covered(&wall, (100, 200), (100, 100)));
        assert!(covered(&wall, (199, 199), (100, 100)) && covered(&wall, (1, 1), (100, 100)));
        assert_eq!(clear_spot(&[], &wall, (100, 100), (0, 0, 300, 300)), Some((0, 0)));
        assert_eq!(clear_spot(&[(0, 0), (0, 100), (0, 200)], &wall, (100, 100), (0, 0, 300, 300)), Some((100, 0)));
        assert_eq!(clear_spot(&[], &[(0, 0, 300, 300)], (100, 100), (0, 0, 300, 300)), None);
        assert_eq!(clear_spot(&[], &[], (100, 100), (50, 20, 250, 250)), Some((50, 20)));
        assert_eq!(corner_slot(&[], (100, 100), (50, 20, 300, 200)), (250, 120));
        assert_eq!(corner_slot(&[(250, 120)], (100, 100), (50, 20, 300, 200)), (250, 20));
        assert_eq!(corner_slot(&[(250, 120), (250, 20)], (100, 100), (50, 20, 300, 200)), (150, 120));
        assert_eq!(free_slots(&[], &[], (100, 50), (10, 10, 300, 120), 3), vec![(10, 10), (10, 60), (110, 10)]);
    }

    #[test]
    fn a_full_or_too_small_area_still_gives_a_sensible_cell() {
        assert_eq!((clear_spot(&[], &[], (100, 100), (0, 0, 50, 300)), clear_spot(&[], &[], (100, 100), (0, 0, 300, 50))), (None, None));
        let order = [(0, 0), (0, 100), (0, 200), (100, 0), (100, 100), (100, 200), (200, 0)];
        assert_eq!(clear_spot(&order, &[], (100, 100), (0, 0, 300, 300)), Some((200, 100)));
        let all = [(200, 200), (200, 100), (200, 0), (100, 200), (100, 100), (100, 0), (0, 200), (0, 100)];
        assert_eq!(corner_slot(&all, (100, 100), (0, 0, 300, 300)), (0, 0));
        let full = [(250, 120), (250, 20), (150, 120), (150, 20), (50, 120), (50, 20)];
        assert_eq!(corner_slot(&full, (100, 100), (50, 20, 300, 200)), (250, 120));
    }
}
