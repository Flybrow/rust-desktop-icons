use super::rows::{Act, Ctl, ITEM, Row, adapt_label, header, note, row, speed_label, total};
use crate::{
    app::App,
    domain::{Look, hotkey::Slot, icons},
    i18n::{self, T},
    layered::glyph as g,
    prefs,
};

pub const PAGES: [(T, char); 8] = [
    (T::Appearance, g::COLOR),
    (T::IconsPage, g::SIZE),
    (T::TitleBar, g::TAB),
    (T::Behavior, g::TAB),
    (T::Desktop, g::PC),
    (T::Rules, g::SYNC),
    (T::Shortcuts, g::KEYBOARD),
    (T::General, g::SETTINGS),
];

fn capturing(a: &App) -> Option<Slot> {
    a.panel.as_ref().and_then(|p| p.capture)
}

fn key_row(a: &App, s: Slot) -> Row {
    let label = a.cfg.key(s).map_or_else(|| a.t(T::None).to_string(), |k| k.label());
    let value = match capturing(a) {
        Some(c) if c == s => "…".into(),
        _ if crate::visibility::hotkeys::failed(s) => format!("⚠ {label}"),
        _ => label,
    };
    let (name, desc, glyph) = match s {
        Slot::Peek => (T::PeekKey, T::DPeekKey, g::KEYBOARD),
        Slot::Search => (T::Search, T::DSearchKey, g::SEARCH),
        Slot::HideAll => (T::HideAllKey, T::DHideKey, g::HIDE),
        Slot::NewFence => (T::NewFence, T::DNewKey, g::ADD),
        Slot::Settings => (T::Settings, T::DSettingsKey, g::SETTINGS),
    };
    let reset = (a.cfg.key(s) != s.default_key()).then_some(Act::ResetKey(s));
    Row { reset, ..row(a.t(name), Ctl::Value(value), Act::Key(s), glyph).about(a.t(desc)) }
}

fn rules(a: &App) -> Vec<Row> {
    let c = &a.cfg;
    let mut rows = vec![row(a.t(T::AutoSort), Ctl::Toggle(c.auto_sort), Act::AutoSort, g::SYNC).about(a.t(T::DAutoSort))];
    for (f, t, place) in c.targets() {
        let kinds: Vec<&str> = c.tab(f, t).map_or(vec![], |t| t.kinds.iter().map(|k| a.t(i18n::kind(*k))).collect());
        let value = if kinds.is_empty() { a.t(T::None).into() } else { kinds.join(", ") };
        rows.push(row(&place, Ctl::Value(value), Act::Rule(f, t), g::FOLDER).about(a.t(T::DRule)));
    }
    rows.push(row(a.t(T::SortNow), Ctl::Button, Act::SortNow, g::SORT).about(a.t(T::DSortNow)));
    rows.push(header(a.t(T::RulesByName)));
    for (i, r) in c.name_rules.iter().enumerate() {
        rows.push(row(&r.pattern, Ctl::Value(format!("→ {}", c.place_name(r.fence, r.tab))), Act::NameRule(i), g::RENAME));
    }
    rows.push(row(a.t(T::AddNameRule), Ctl::Button, Act::AddNameRule, g::ADD).about(a.t(T::DNameRule)));
    rows.push(header(a.t(T::RulesByAge)));
    for (f, t, e) in c.expiring() {
        let label = format!("{} · {}", c.place_name(f, t), a.t(T::OlderThan).replace("{}", &e.days.to_string()));
        rows.push(row(&label, Ctl::Value(format!("→ {}", c.place_name(e.fence, e.tab))), Act::AgeRule(f, t), g::SPEED));
    }
    rows.push(row(a.t(T::AddAgeRule), Ctl::Button, Act::AddAgeRule, g::ADD).about(a.t(T::DAgeRule)));
    rows
}

pub fn build(a: &App, page: usize) -> Vec<Row> {
    let (c, l) = (&a.cfg, &a.cfg.look);
    let mut rows = vec![header(a.t(PAGES[page.min(PAGES.len() - 1)].0))];
    rows.extend(match page {
        0 => vec![
            note(a.t(T::LookNote)),
            row(a.t(T::Color), Ctl::Swatch(Some(l.color)), Act::Color, g::COLOR).about(a.t(T::DColor)),
            row(a.t(T::Wallpaper), Ctl::Toggle(l.wallpaper), Act::Wallpaper, g::PICTURE).about(a.t(T::DWallpaper)),
            row(a.t(T::Opacity), Ctl::Value(Look::label(l.percent())), Act::Opacity, g::OPACITY).about(a.t(T::DOpacity)),
            row(a.t(T::Tint), Ctl::Swatch(l.tint), Act::Tint, g::TINT).about(a.t(T::DTint)),
            row(a.t(T::ApplyAll), Ctl::Button, Act::ApplyAll, g::CHECK).about(a.t(T::DApplyAll)),
        ],
        1 => vec![
            note(a.t(T::LookNote)),
            row(a.t(T::IconSize), Ctl::Value(icons::label(l.icon)), Act::IconSize, g::SIZE).about(a.t(T::DIconSize)),
            row(a.t(T::Labels), Ctl::Toggle(l.labels), Act::Labels, g::LABEL).about(a.t(T::DLabels)),
            row(a.t(T::Thumbs), Ctl::Toggle(l.thumbs), Act::Thumbs, g::IMAGE).about(a.t(T::DThumbs)),
            row(a.t(T::LabelSize), Ctl::Value(a.t(i18n::label_size(l.label_size)).into()), Act::LabelSize, g::RENAME)
                .about(a.t(T::DLabelSize)),
            row(a.t(T::LabelColor), Ctl::Swatch(l.label_color), Act::LabelColor, g::COLOR).about(a.t(T::DLabelColor)),
            row(a.t(T::Spacing), Ctl::Value(a.t(i18n::spacing(l.spacing)).into()), Act::Spacing, g::HEIGHT).about(a.t(T::DSpacing)),
            row(a.t(T::Align), Ctl::Value(a.t(i18n::align(l.align)).into()), Act::Align, g::ALIGN).about(a.t(T::DAlign)),
        ],
        2 => vec![
            note(a.t(T::LookNote)),
            row(a.t(T::TitleAlign), Ctl::Value(a.t(i18n::align(l.title_align)).into()), Act::TitleAlign, g::ALIGN)
                .about(a.t(T::DTitleAlign)),
            row(a.t(T::TitleSize), Ctl::Value(a.t(i18n::label_size(l.title_size)).into()), Act::TitleSize, g::FONT)
                .about(a.t(T::DTitleSize)),
            row(a.t(T::BarColor), Ctl::Swatch(l.bar_color), Act::BarColor, g::COLOR).about(a.t(T::DBarColor)),
            row(a.t(T::BarHover), Ctl::Toggle(l.bar_hover), Act::BarHover, g::HIDE).about(a.t(T::DBarHover)),
        ],
        3 => vec![
            note(a.t(T::LookNote)),
            row(a.t(T::Chameleon), Ctl::Toggle(l.chameleon), Act::Chameleon, g::EYE).about(a.t(T::DChameleon)),
            row(a.t(T::AutoHeight), Ctl::Toggle(l.auto_height), Act::AutoHeight, g::HEIGHT).about(a.t(T::DAutoHeight)),
            row(a.t(T::Browse), Ctl::Toggle(l.browse), Act::Browse, g::FOLDER).about(a.t(T::DBrowse)),
            row(a.t(T::RollSpeed), Ctl::Value(speed_label(a, c.roll_ms)), Act::Speed, g::SPEED).about(a.t(T::DRollSpeed)),
            note(a.t(T::SnapHint)),
        ],
        4 => vec![
            row(a.t(T::KeepOnDesktop), Ctl::Toggle(c.keep_on_desktop), Act::KeepOnDesktop, g::FOLDER).about(a.t(T::DKeep)),
            row(a.t(T::ScreenChange), Ctl::Value(adapt_label(a, c.screen_adapt).into()), Act::Screens, g::PC).about(a.t(T::DScreens)),
            row(a.t(T::DblClickHide), Ctl::Toggle(c.dblclick_hide), Act::DblClick, g::HIDE).about(a.t(T::DDblClick)),
        ],
        5 => rules(a),
        6 => Slot::ALL.iter().map(|s| key_row(a, *s)).chain(capturing(a).map(|_| note(a.t(T::PressKeys)))).collect(),
        _ => vec![
            row(a.t(T::Language), Ctl::Value(prefs::lang_name(a).into()), Act::Lang, g::GLOBE).about(a.t(T::DLanguage)),
            row(a.t(T::Autostart), Ctl::Toggle(c.autostart), Act::Autostart, g::POWER).about(a.t(T::DAutostart)),
            row(a.t(T::AutoUpdate), Ctl::Toggle(c.auto_update), Act::AutoUpdate, g::DOWNLOAD).about(a.t(T::DAutoUpdate)),
            row(a.t(T::CheckUpdates), Ctl::Value(format!("v{}", env!("CARGO_PKG_VERSION"))), Act::Check, g::REFRESH).about(a.t(T::DCheck)),
            row(a.t(T::Export), Ctl::Button, Act::Export, g::DOWNLOAD).about(a.t(T::DExport)),
            row(a.t(T::Import), Ctl::Button, Act::Import, g::FOLDER).about(a.t(T::DImport)),
        ],
    });
    rows
}

pub fn tallest(a: &App) -> i32 {
    (0..PAGES.len()).map(|p| total(&build(a, p))).max().unwrap_or(0).max(PAGES.len() as i32 * ITEM)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        Config, Fence, Tab,
        matching::{Expire, NameRule},
    };

    fn app() -> App {
        let mut a = Fence::new(1, "Docs", (0, 0, 100, 100), Look::default());
        a.tabs[0].expire = Some(Expire { days: 30, fence: 2, tab: 2 });
        a.tabs.push(Tab { id: 5, title: "Web".into(), portal: Some("/w".into()), ..Tab::default() });
        let fences = vec![a, Fence::new(2, "Archive", (0, 0, 100, 100), Look::default())];
        let name_rules = vec![NameRule { pattern: "invoice*".into(), fence: 2, tab: 2 }];
        App::new(Config { lang: Some("en".into()), fences, name_rules, ..Config::default() }, 0)
    }

    #[test]
    fn every_option_is_on_one_page_with_an_explanation() {
        let a = app();
        let pages: Vec<Vec<Row>> = (0..PAGES.len()).map(|p| build(&a, p)).collect();
        assert!(pages.iter().zip(PAGES).all(|(rows, (t, _))| rows[0].label == a.t(t) && rows.len() > 1));
        let acts: Vec<Act> = pages.iter().flatten().filter_map(|r| r.act).collect();
        let fixed = [
            Act::Color,
            Act::Opacity,
            Act::Tint,
            Act::ApplyAll,
            Act::IconSize,
            Act::Labels,
            Act::LabelSize,
            Act::LabelColor,
            Act::Spacing,
            Act::Align,
            Act::Thumbs,
            Act::Wallpaper,
            Act::TitleAlign,
            Act::TitleSize,
            Act::BarColor,
            Act::BarHover,
            Act::Chameleon,
            Act::AutoHeight,
            Act::Browse,
            Act::Speed,
            Act::KeepOnDesktop,
            Act::Screens,
            Act::DblClick,
            Act::AutoSort,
            Act::SortNow,
            Act::AddNameRule,
            Act::AddAgeRule,
            Act::Lang,
            Act::Autostart,
            Act::AutoUpdate,
            Act::Check,
            Act::Export,
            Act::Import,
        ];
        for act in fixed.into_iter().chain(Slot::ALL.map(Act::Key)) {
            assert_eq!(acts.iter().filter(|x| **x == act).count(), 1, "{act:?}");
        }
        let dynamic = |a: &Act| matches!(a, Act::NameRule(_) | Act::AgeRule(..));
        assert!(pages.iter().flatten().filter(|r| r.act.is_some_and(|x| !dynamic(&x))).all(|r| !r.label.is_empty() && !r.desc.is_empty()));
        assert!(tallest(&a) >= total(&pages[5]));
    }

    #[test]
    fn rules_page_lists_tabs_and_both_rule_kinds() {
        let a = app();
        let rows = build(&a, 5);
        let acts: Vec<Act> = rows.iter().filter_map(|r| r.act).collect();
        assert!(acts.contains(&Act::Rule(1, 1)) && acts.contains(&Act::Rule(2, 2)) && !acts.contains(&Act::Rule(1, 5)));
        let name = rows.iter().find(|r| r.act == Some(Act::NameRule(0))).unwrap();
        assert_eq!((name.label.as_str(), name.ctl.clone()), ("invoice*", Ctl::Value("→ Archive".into())));
        let age = rows.iter().find(|r| r.act == Some(Act::AgeRule(1, 1))).unwrap();
        assert!(age.label.contains("30") && age.ctl == Ctl::Value("→ Archive".into()));
    }

    #[test]
    fn shortcuts_show_their_keys_and_a_reset_when_changed() {
        let mut a = app();
        a.cfg.peek_key = None;
        let rows = build(&a, 6);
        let peek = rows.iter().find(|r| r.act == Some(Act::Key(Slot::Peek))).unwrap();
        let hide = rows.iter().find(|r| r.act == Some(Act::Key(Slot::HideAll))).unwrap();
        assert_eq!((peek.reset, hide.reset), (Some(Act::ResetKey(Slot::Peek)), None));
        assert_eq!(hide.ctl, Ctl::Value(a.t(T::None).into()));
    }
}
