use super::{Config, Fence, Kind, Sort, inherit::Field, system::Sys};
use std::path::Path;

pub(super) const FIXTURES: [(&str, &str); 5] = [
    ("0.1.0", include_str!("fixtures/v0.1.0.json")),
    ("0.7.0", include_str!("fixtures/v0.7.0.json")),
    ("0.7.14", include_str!("fixtures/v0.7.14.json")),
    ("0.8.1", include_str!("fixtures/v0.8.1.json")),
    ("0.9.4", include_str!("fixtures/v0.9.4.json")),
];

fn load(v: &str) -> Config {
    let json = FIXTURES.iter().find(|(n, _)| *n == v).unwrap().1;
    Config::parse(json.as_bytes()).unwrap_or_else(|e| panic!("{v}: {e}")).sanitized()
}

fn fence(c: &Config, id: u64) -> &Fence {
    c.fence(id).unwrap()
}

fn custom(f: &Fence) -> Vec<Field> {
    f.custom.clone().unwrap_or_default()
}

#[test]
fn every_released_config_loads_and_stays_stable() {
    for (v, _) in FIXTURES {
        let c = load(v);
        assert!(!c.fences.is_empty(), "{v}");
        let again = Config::parse(&serde_json::to_vec(&c).unwrap()).unwrap().sanitized();
        assert_eq!(again, c, "{v}: saving then loading changes the config");
        assert_eq!(c.clone().sanitized(), c, "{v}: sanitizing twice changes the config");
        for f in &c.fences {
            assert!(f.tab < f.tabs.len() && f.tabs.iter().all(|t| t.id > 0) && f.title.is_empty(), "{v}");
        }
    }
}

#[test]
fn v0_1_titles_colors_and_choices_survive() {
    let c = load("0.1.0");
    assert_eq!((c.lang.as_deref(), c.autostart, c.auto_update), (Some("fr"), false, true));
    let f = fence(&c, 1);
    assert_eq!((f.x, f.y, f.w, f.h, f.rolled), (40, 60, 420, 260, true));
    assert_eq!((f.active().title.as_str(), f.look.color, f.look.alpha), ("Jeux", 1315860, 200));
    assert_eq!(custom(f), vec![Field::Color, Field::Alpha]);
    assert_eq!((fence(&c, 2).active().title.as_str(), custom(fence(&c, 2))), ("Documents", vec![]));
}

#[test]
fn v0_7_look_tabs_and_portals_survive() {
    let c = load("0.7.0");
    assert_eq!((c.lang.as_deref(), c.auto_update, c.auto_sort), (Some("de"), false, false));
    assert_eq!((c.look.color, c.look.alpha, c.look.icon, c.look.tint, c.look.chameleon), (2105376, 230, 64, Some(16711680), true));
    let f = fence(&c, 4);
    assert_eq!((f.tab, f.tabs.len(), custom(f)), (1, 2, vec![]));
    assert_eq!(f.tabs[0].kinds, vec![Kind::Games, Kind::Apps]);
    assert_eq!(f.tabs[1].portal.as_deref(), Some(Path::new("D:\\Downloads")));
    assert_eq!(f.look, c.look);
    let c = load("0.7.14");
    let f = fence(&c, 1);
    assert_eq!((f.locked, f.look.color, f.look.icon, f.look.auto_height), (true, 255, 32, false));
    assert_eq!(custom(f), vec![Field::Color, Field::Icon, Field::AutoHeight]);
    assert_eq!((f.tabs[0].sort, f.tabs[0].order.clone()), (Sort::Custom, vec!["b.txt".to_string(), "a.txt".to_string()]));
    assert_eq!((f.tabs[0].system.clone(), f.tabs[1].system.clone()), (vec![Sys::RecycleBin], vec![]));
    assert_eq!((f.tabs[1].sort, f.tabs[1].kinds.clone()), (Sort::Date, vec![Kind::Images, Kind::Videos]));
    assert_eq!(c.version.as_deref(), Some("0.7.14"));
}

#[test]
fn v0_8_and_v0_9_desktop_state_survives() {
    let c = load("0.8.1");
    assert_eq!((c.keep_on_desktop, c.desk_flags), (true, Some(1075839525)));
    let t = &fence(&c, 3).tabs[0];
    assert_eq!((t.pins.len(), t.pins[0].id, t.sort), (1, 281474976718001, Sort::Type));
    assert_eq!(t.pins[0].path, Path::new("C:\\Users\\me\\Desktop\\report.docx"));
    assert_eq!((fence(&c, 3).x, fence(&c, 3).rolled), (1920, true));
    let c = load("0.9.4");
    assert_eq!(c.desk_icons, vec![(Sys::RecycleBin, Some(0)), (Sys::ThisPc, None)]);
    let (a, b) = (fence(&c, 1), fence(&c, 2));
    assert_eq!((a.vertical, a.rolled, a.tabs[0].system.clone()), (true, true, vec![Sys::RecycleBin, Sys::ThisPc]));
    assert_eq!((b.look.icon, custom(b), b.tabs[0].portal.as_deref()), (64, vec![Field::Icon], Some(Path::new("C:\\Projets"))));
    assert_eq!((b.tabs[0].kinds.clone(), b.tabs[0].sort), (vec![Kind::Folders], Sort::Date));
}
