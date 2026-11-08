use super::{
    Config, Fence,
    grid::{self, Align, Flow, Move},
    layouts::beside,
    matching::glob,
    screens::{Screen, scale},
    search,
    side::{Geo, Side},
    snap,
};
use proptest::prelude::*;
use serde_json::{Map, Value};
use std::collections::HashSet;

fn align() -> impl Strategy<Value = Align> {
    prop::sample::select(Align::ALL.to_vec())
}

fn side() -> impl Strategy<Value = Side> {
    prop::sample::select(vec![Side::Top, Side::Bottom, Side::Left, Side::Right])
}

fn json() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(Value::from),
        any::<f64>().prop_map(Value::from),
        "[a-zA-Z0-9 _.]{0,12}".prop_map(Value::from),
    ];
    leaf.prop_recursive(4, 48, 8, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(Value::Array),
            prop::collection::vec((key(), inner), 0..8).prop_map(|kv| Value::Object(kv.into_iter().collect::<Map<_, _>>())),
        ]
    })
}

fn key() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "fences",
        "tabs",
        "id",
        "x",
        "y",
        "w",
        "h",
        "tab",
        "title",
        "portal",
        "kinds",
        "sort",
        "order",
        "system",
        "pins",
        "look",
        "color",
        "alpha",
        "icon",
        "rolled",
        "locked",
        "vertical",
        "custom",
        "group",
        "desktop",
        "lang",
        "roll_ms",
        "name_rules",
        "screens",
        "layouts",
        "snapshots",
        "peek_key",
        "expire",
        "filter",
        "path",
        "days",
        "vk",
        "mods",
    ])
    .prop_map(String::from)
}

fn check_invariants(c: &Config) {
    let mut fences = HashSet::new();
    let mut tabs = HashSet::new();
    for f in &c.fences {
        assert!(f.id > 0 && fences.insert(f.id));
        assert!(!f.tabs.is_empty() && f.tab < f.tabs.len());
        assert!(f.w >= Fence::MIN.0 && f.h >= Fence::MIN.1);
        assert!(f.tabs.iter().all(|t| t.id > 0 && tabs.insert(t.id)));
    }
    assert!(c.next_id() > c.fences.iter().flat_map(|f| f.tabs.iter().map(|t| t.id).chain([f.id])).max().unwrap_or(0));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    #[test]
    fn grid_positions_and_hits_agree(w in 1..2500i32, cell in 1..300i32, n in 0..200usize, a in align()) {
        let f = Flow { w, cell, n, align: a };
        let cols = grid::cols(w, cell);
        for i in 0..n {
            let (x, y) = f.origin(i);
            prop_assert_eq!(f.index_at((x + cell / 2, y + cell / 2)), Some(i));
            prop_assert!(x >= 0 && (cell > w || x + cell <= w));
            prop_assert_eq!(y, i as i32 / cols * cell);
        }
        prop_assert_eq!(f.in_rect((0, 0, w.max(cell), grid::extent(n, w, cell, 0) + cell)), (0..n).collect::<Vec<_>>());
        prop_assert_eq!(f.index_at((-1, 0)), None);
    }

    #[test]
    fn keyboard_steps_stay_inside(n in 0..300usize, cols in -2..40i32, at in prop::option::of(0..400usize), r in -50..50i32, fx: bool, fy: bool) {
        for m in [Move::Left, Move::Right, Move::Up, Move::Down, Move::Home, Move::End, Move::Page(r)] {
            let to = grid::step(n, cols, at, m, (fx, fy));
            prop_assert_eq!(to.is_some(), n > 0);
            prop_assert!(to.is_none_or(|i| i < n));
        }
    }

    #[test]
    fn bar_side_mapping_round_trips(s in side(), w in 50..3000i32, h in 50..3000i32, bar in 0..60i32, gap in 0..30i32, x in -500..3000i32, y in -500..3000i32) {
        let g = Geo { side: s, size: (w, h), bar, gap };
        prop_assert_eq!(g.to_content(g.to_window((x, y))), (x, y));
        prop_assert_eq!(g.to_window(g.to_content((x, y))), (x, y));
        let (b, body) = (g.bar_rect(), g.body());
        let area = |r: (i32, i32, i32, i32)| (r.2 - r.0) as i64 * (r.3 - r.1) as i64;
        prop_assert_eq!(area(b) + area(body), w as i64 * h as i64);
    }

    #[test]
    fn scaled_fences_land_on_the_new_screen(x in -100..1800i32, y in -100..1000i32, w in 120..1900i32, h in 80..1000i32, nw in 800..4000i32, nh in 600..2200i32) {
        let screen = |w: i32, h: i32| Screen { full: [0, 0, w, h], work: [0, 0, w, h - 40] };
        let (old, now) = ([screen(1920, 1080)], [screen(nw, nh)]);
        let [sx, sy, sw, sh] = scale([x, y, w, h], &old, &now);
        let work = now[0].work;
        prop_assert!(sw <= work[2] - work[0] && sh <= work[3] - work[1]);
        prop_assert!(sx >= work[0] && sx + sw <= work[2] && sy >= work[1] && sy + sh <= work[3]);
    }

    #[test]
    fn a_copy_never_covers_another_fence(fx in 0..1500i32, fy in 0..700i32, fw in 120..420i32, fh in 80..320i32,
        others in prop::collection::vec((0..1600i32, 0..800i32, 120..400i32, 80..300i32), 0..3)) {
        let f = Fence { x: fx, y: fy, w: fw, h: fh, ..Fence::default() };
        let mut rects = vec![[fx, fy, fw, fh]];
        rects.extend(others.iter().map(|&(x, y, w, h)| [x, y, w, h]));
        let (x, y) = beside(&f, [0, 0, 1920, 1040], 10, &rects);
        prop_assert!(x >= 0 && y >= 0 && x + fw <= 1920 && y + fh <= 1040);
        prop_assert!(rects.iter().all(|&[ox, oy, ow, oh]| x + fw + 10 <= ox || ox + ow + 10 <= x || y + fh + 10 <= oy || oy + oh + 10 <= y));
    }

    #[test]
    fn snapped_moves_stay_on_screen(x in 0..1500i32, y in 0..700i32, w in 120..400i32, h in 80..300i32,
        others in prop::collection::vec((0..1600i32, 0..800i32, 120..400i32, 80..300i32), 0..5)) {
        let area = [0, 0, 1920, 1040];
        let others: Vec<[i32; 4]> = others.iter().map(|&(x, y, w, h)| [x, y, x + w, y + h]).collect();
        let (r, _) = snap::moving([x, y, x + w, y + h], area, &others, 12, 10);
        prop_assert_eq!((r[2] - r[0], r[3] - r[1]), (w, h));
        prop_assert!((r[0] - x).abs() <= 12 && (r[1] - y).abs() <= 12);
    }

    #[test]
    fn globs_match_like_explorer(name in "[a-zA-Z0-9 ._-]{0,20}", cut in 0..20usize) {
        let cut = name.char_indices().map(|(i, _)| i).chain([name.len()]).nth(cut.min(name.chars().count())).unwrap_or(0);
        let (head, tail) = name.split_at(cut);
        prop_assert!(glob("*", &name) && glob(&name, &name) && glob(&name.to_uppercase(), &name));
        let (starts, ends, longer) = (format!("{head}*"), format!("*{tail}"), format!("{name}x"));
        prop_assert!(glob(&starts, &name) && glob(&ends, &name));
        prop_assert!(glob(&"?".repeat(name.chars().count()), &name));
        prop_assert!(!glob(&longer, &name));
    }

    #[test]
    fn search_returns_real_matches_best_first(names in prop::collection::vec("[a-zA-Zéèà ]{1,12}", 0..30), q in "[a-zA-Zé]{1,3}") {
        let hits = search::rank(&q, &names);
        let mut seen = HashSet::new();
        let nq = search::normalize(&q);
        for &i in &hits {
            prop_assert!(i < names.len() && seen.insert(i) && search::normalize(&names[i]).contains(&nq));
        }
        prop_assert_eq!(search::normalize(&search::normalize(&q)), nq.clone());
        if let Some(first) = names.iter().position(|n| search::normalize(n) == nq) {
            prop_assert!(hits.first().is_some_and(|&i| search::normalize(&names[i]).starts_with(&nq)), "{} not first", first);
        }
    }

    #[test]
    fn clashing_ids_are_always_made_unique(fences in prop::collection::vec((0..6u64, prop::collection::vec(0..6u64, 0..4), 0..6usize), 0..6)) {
        let fences: Vec<Value> = fences.iter().map(|(id, tabs, tab)| serde_json::json!({
            "id": id, "tab": tab, "tabs": tabs.iter().map(|t| serde_json::json!({"id": t})).collect::<Vec<_>>()
        })).collect();
        let c = Config::parse(&serde_json::to_vec(&serde_json::json!({"fences": fences})).unwrap()).unwrap().sanitized();
        check_invariants(&c);
        prop_assert_eq!(c.clone().sanitized(), c);
    }

    #[test]
    fn any_json_loads_safely(v in json()) {
        if let Ok(c) = Config::parse(&serde_json::to_vec(&v).unwrap()) {
            let c = c.sanitized();
            check_invariants(&c);
            prop_assert_eq!(c.clone().sanitized(), c);
        }
    }

    #[test]
    fn damaged_real_configs_load_safely(which in 0..5usize, path in prop::collection::vec(0..12usize, 1..5), v in json()) {
        let mut root: Value = serde_json::from_str(super::upgrade::FIXTURES[which].1).unwrap();
        let mut pointer = String::new();
        for k in path {
            let step = match root.pointer(&pointer) {
                Some(Value::Object(m)) if !m.is_empty() => m.keys().nth(k % m.len()).cloned(),
                Some(Value::Array(a)) if !a.is_empty() => Some((k % a.len()).to_string()),
                _ => None,
            };
            match step { Some(s) => pointer = format!("{pointer}/{}", s.replace('~', "~0").replace('/', "~1")), None => break }
        }
        if let Some(at) = root.pointer_mut(&pointer) { *at = v; }
        if let Ok(c) = Config::parse(&serde_json::to_vec(&root).unwrap()) {
            let c = c.sanitized();
            check_invariants(&c);
            prop_assert_eq!(c.clone().sanitized(), c);
        }
    }
}

#[test]
fn clashing_ids_never_share_a_folder() {
    let json = r#"{"fences":[{"id":1,"tabs":[{"id":2}]},{"id":2,"tabs":[{"id":2}]},{"id":3,"tabs":[{"id":1}]}]}"#;
    let c = Config::parse(json.as_bytes()).unwrap().sanitized();
    check_invariants(&c);
}
