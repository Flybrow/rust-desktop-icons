use super::*;

#[test]
fn a_list_is_one_full_width_column() {
    let f = Flow { w: 300, cell: 40, cw: 300, n: 4, align: Align::Center };
    assert_eq!((f.origin(0), f.origin(3)), ((0, 0), (0, 120)));
    assert_eq!((f.index_at((290, 45)), f.index_at((5, 119)), f.index_at((5, 160))), (Some(1), Some(2), None));
    assert_eq!(f.in_rect((250, 30, 260, 50)), vec![0, 1]);
    assert_eq!((extent(4, 40, 40, 40), last_row(4, 40, 40)), (160, 3..4));
    let squeezed = Flow { w: 0, cell: 40, cw: 0, n: 3, align: Align::Left };
    assert_eq!((squeezed.origin(2), squeezed.index_at((0, 45)), cols(0, 0)), ((0, 80), Some(1), 1));
}

#[test]
fn grid_layout() {
    let flow = |n, align| Flow { w: 250, cell: 80, cw: 80, n, align };
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
    let flow = |n| Flow { w: 250, cell: 80, cw: 80, n, align: Align::Center };
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
    let flow = |n| Flow { w: 250, cell: 80, cw: 80, n, align: Align::Left };
    assert_eq!((flow(5).index_at((170, 90)), flow(5).index_at((90, 90))), (None, Some(4)));
    assert_eq!((flow(3).in_rect((0, 0, 80, 80)), flow(5).in_rect((0, 0, 250, 80))), (vec![0], vec![0, 1, 2]));
    assert!(flow(5).in_rect((160, 160, 300, 300)).is_empty() && flow(5).in_rect((0, 160, 250, 300)).is_empty());
    assert_eq!(step(8, 3, Some(3), Move::Down, (false, false)), Some(6));
}
