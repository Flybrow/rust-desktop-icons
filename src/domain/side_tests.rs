use super::*;

fn geo(side: Side) -> Geo {
    Geo { side, size: (300, 200), bar: 30, gap: 4, head: 0 }
}

#[test]
fn a_header_sits_between_the_bar_and_the_rows() {
    let g = |side| Geo { head: 20, ..geo(side) };
    assert_eq!((g(Side::Top).head_rect(), g(Side::Top).rows(), g(Side::Top).view_h()), ((0, 34, 300, 54), (0, 54, 300, 200), 146));
    assert_eq!((g(Side::Bottom).head_rect(), g(Side::Bottom).rows()), ((0, 146, 300, 166), (0, 0, 300, 146)));
    assert_eq!((g(Side::Left).head_rect(), g(Side::Left).rows(), g(Side::Left).view_h()), ((34, 0, 300, 20), (30, 20, 300, 200), 180));
    assert_eq!((g(Side::Right).head_rect(), g(Side::Right).rows()), ((0, 0, 266, 20), (0, 20, 270, 200)));
    assert_eq!((geo(Side::Top).rows(), geo(Side::Top).head_rect()), (geo(Side::Top).body(), (0, 34, 300, 34)));
    for side in [Side::Top, Side::Bottom, Side::Left, Side::Right] {
        assert_eq!(g(side).to_content(g(side).to_window((7, 9))), (7, 9));
        assert_eq!(g(side).to_content(g(side).to_window((7, -5))).1, -5);
    }
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
    assert_eq!(geo(Side::Bottom).cell((0, 0), (50, 50)), (0, 116));
    assert_eq!(geo(Side::Right).cell((0, 0), (50, 50)), (216, 0));
    assert_eq!(geo(Side::Left).cell((50, 50), (50, 50)), (84, 50));
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

#[test]
fn far_also_works_on_a_screen_that_does_not_start_at_zero() {
    let area = (1000, 2040);
    assert!(far(1520, 34, 600, area, 16) && !far(1100, 34, 1000, area, 16));
    assert!(!far(1520, 34, 504, area, 16) && !far(1503, 34, 600, area, 16) && far(1520, 34, 520, area, 16));
    assert!(!Side::Top.far() && !Side::Left.far() && Side::Bottom.far());
}
