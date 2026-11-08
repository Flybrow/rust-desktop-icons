use crate::{
    app::App,
    domain::{hotkey::Slot, screens::Adapt},
    i18n::T,
    layered::glyph as g,
};

pub const TOP: i32 = crate::layered::chrome::TOP;
pub const SIDE: i32 = 190;
pub const WIDTH: i32 = SIDE + 460;

pub use super::layout::{ITEM, RESET_LINE, SECOND_LINE, cards, hit, layout, on_reset, page_at, total};

pub use super::pages::{PAGES, build, tallest};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Act {
    Color,
    Opacity,
    IconSize,
    Tint,
    Labels,
    LabelSize,
    LabelColor,
    Spacing,
    Align,
    Browse,
    Thumbs,
    Wallpaper,
    TitleAlign,
    TitleSize,
    BarColor,
    BarHover,
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
    NameRule(usize),
    AddNameRule,
    AgeRule(u64, u64),
    AddAgeRule,
    Export,
    Import,
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
