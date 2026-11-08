use crate::{
    app::App,
    domain::{Look, hotkey::Slot, icons, screens::Adapt},
    i18n::{self, T},
    layered::glyph as g,
    prefs,
};

pub const TOP: i32 = crate::layered::chrome::TOP;
pub const SIDE: i32 = 190;
pub const WIDTH: i32 = SIDE + 460;

pub use super::layout::{ITEM, RESET_LINE, SECOND_LINE, cards, hit, layout, on_reset, page_at, total};

pub const PAGES: [(T, char); 5] =
    [(T::Appearance, g::COLOR), (T::Behavior, g::TAB), (T::Desktop, g::PC), (T::Rules, g::SYNC), (T::General, g::SETTINGS)];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Act {
    Color,
    Opacity,
    IconSize,
    Tint,
    Labels,
    Align,
    Chameleon,
    AutoHeight,
    Speed,
    KeepOnDesktop,
    Screens,
    DblClick,
    Key(Slot),
    ResetKey(Slot),
    ApplyAll,
    AutoSort,
    Rule(u64, u64),
    SortNow,
    Lang,
    Autostart,
    AutoUpdate,
    Check,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ctl {
    Header,
    Toggle(bool),
    Value(String),
    Swatch(Option<u32>),
    Button,
    Note,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub label: String,
    pub ctl: Ctl,
    pub act: Option<Act>,
    pub glyph: char,
    pub desc: String,
    pub reset: Option<Act>,
}

impl Row {
    pub(super) fn about(self, desc: &str) -> Row {
        Row { desc: desc.into(), ..self }
    }
}

fn plain(label: &str, ctl: Ctl, act: Option<Act>, glyph: char) -> Row {
    Row { label: label.trim_end_matches('…').into(), ctl, act, glyph, desc: String::new(), reset: None }
}

pub(super) fn row(label: &str, ctl: Ctl, act: Act, glyph: char) -> Row {
    plain(label, ctl, Some(act), glyph)
}

pub(super) fn header(label: &str) -> Row {
    plain(label, Ctl::Header, None, ' ')
}

pub(super) fn note(label: &str) -> Row {
    plain(label, Ctl::Note, None, g::INFO)
}

pub fn speed_label(a: &App, ms: u32) -> String {
    if ms == 0 { a.t(T::Instant).into() } else { format!("{ms} ms") }
}

pub fn adapt_label(a: &App, m: Adapt) -> &'static str {
    a.t(match m {
        Adapt::Keep => T::ScreenKeep,
        Adapt::Scale => T::ScreenScale,
        Adapt::Remember => T::ScreenRemember,
    })
}

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
    let (name, desc, glyph) = if s == Slot::Peek { (T::PeekKey, T::DPeekKey, g::KEYBOARD) } else { (T::Search, T::DSearchKey, g::SEARCH) };
    let reset = (a.cfg.key(s) != Some(s.default_key())).then_some(Act::ResetKey(s));
    Row { reset, ..row(a.t(name), Ctl::Value(value), Act::Key(s), glyph).about(a.t(desc)) }
}

fn rules(a: &App) -> Vec<Row> {
    let mut rows = vec![row(a.t(T::AutoSort), Ctl::Toggle(a.cfg.auto_sort), Act::AutoSort, g::SYNC).about(a.t(T::DAutoSort))];
    for f in &a.cfg.fences {
        for t in f.tabs.iter().filter(|t| t.portal.is_none()) {
            let name = if t.id == f.tabs[0].id { t.title.clone() } else { format!("{} › {}", f.tabs[0].title, t.title) };
            let kinds: Vec<&str> = t.kinds.iter().map(|k| a.t(i18n::kind(*k))).collect();
            let value = if kinds.is_empty() { a.t(T::None).into() } else { kinds.join(", ") };
            rows.push(row(&name, Ctl::Value(value), Act::Rule(f.id, t.id), g::FOLDER).about(a.t(T::DRule)));
        }
    }
    rows.push(row(a.t(T::SortNow), Ctl::Button, Act::SortNow, g::SORT).about(a.t(T::DSortNow)));
    rows
}

pub fn build(a: &App, page: usize) -> Vec<Row> {
    let (c, l) = (&a.cfg, &a.cfg.look);
    let mut rows = vec![header(a.t(PAGES[page.min(PAGES.len() - 1)].0))];
    rows.extend(match page {
        0 => vec![
            note(a.t(T::LookNote)),
            row(a.t(T::Color), Ctl::Swatch(Some(l.color)), Act::Color, g::COLOR).about(a.t(T::DColor)),
            row(a.t(T::Opacity), Ctl::Value(Look::label(l.percent())), Act::Opacity, g::OPACITY).about(a.t(T::DOpacity)),
            row(a.t(T::IconSize), Ctl::Value(icons::label(l.icon)), Act::IconSize, g::SIZE).about(a.t(T::DIconSize)),
            row(a.t(T::Tint), Ctl::Swatch(l.tint), Act::Tint, g::TINT).about(a.t(T::DTint)),
            row(a.t(T::Labels), Ctl::Toggle(l.labels), Act::Labels, g::LABEL).about(a.t(T::DLabels)),
            row(a.t(T::Align), Ctl::Value(a.t(i18n::align(l.align)).into()), Act::Align, g::ALIGN).about(a.t(T::DAlign)),
            row(a.t(T::ApplyAll), Ctl::Button, Act::ApplyAll, g::CHECK).about(a.t(T::DApplyAll)),
        ],
        1 => vec![
            note(a.t(T::LookNote)),
            row(a.t(T::Chameleon), Ctl::Toggle(l.chameleon), Act::Chameleon, g::EYE).about(a.t(T::DChameleon)),
            row(a.t(T::AutoHeight), Ctl::Toggle(l.auto_height), Act::AutoHeight, g::HEIGHT).about(a.t(T::DAutoHeight)),
            row(a.t(T::RollSpeed), Ctl::Value(speed_label(a, c.roll_ms)), Act::Speed, g::SPEED).about(a.t(T::DRollSpeed)),
            key_row(a, Slot::Peek),
            key_row(a, Slot::Search),
        ]
        .into_iter()
        .chain(capturing(a).map(|_| note(a.t(T::PressKeys))))
        .chain([note(a.t(T::SnapHint))])
        .collect(),
        2 => vec![
            row(a.t(T::KeepOnDesktop), Ctl::Toggle(c.keep_on_desktop), Act::KeepOnDesktop, g::FOLDER).about(a.t(T::DKeep)),
            row(a.t(T::ScreenChange), Ctl::Value(adapt_label(a, c.screen_adapt).into()), Act::Screens, g::PC).about(a.t(T::DScreens)),
            row(a.t(T::DblClickHide), Ctl::Toggle(c.dblclick_hide), Act::DblClick, g::HIDE).about(a.t(T::DDblClick)),
        ],
        3 => rules(a),
        _ => vec![
            row(a.t(T::Language), Ctl::Value(prefs::lang_name(a).into()), Act::Lang, g::GLOBE).about(a.t(T::DLanguage)),
            row(a.t(T::Autostart), Ctl::Toggle(c.autostart), Act::Autostart, g::POWER).about(a.t(T::DAutostart)),
            row(a.t(T::AutoUpdate), Ctl::Toggle(c.auto_update), Act::AutoUpdate, g::DOWNLOAD).about(a.t(T::DAutoUpdate)),
            row(a.t(T::CheckUpdates), Ctl::Value(format!("v{}", env!("CARGO_PKG_VERSION"))), Act::Check, g::REFRESH).about(a.t(T::DCheck)),
        ],
    });
    rows
}

pub fn tallest(a: &App) -> i32 {
    (0..PAGES.len()).map(|p| total(&build(a, p))).max().unwrap_or(0).max(PAGES.len() as i32 * ITEM)
}
