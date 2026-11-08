use super::{refresh, rows};
use crate::{
    app::{change, with},
    domain::{Kind, Look, anim, grid, icons, screens::Adapt},
    i18n::{self, T},
    keep, prefs, rules,
    tray::{self, updates},
    visibility,
    win::*,
};
use windows::Win32::Foundation::HWND;

pub(super) fn choose(h: HWND, options: Vec<(String, bool)>) -> Option<usize> {
    let rtl = with(|a| a.rtl()).unwrap_or(false);
    let m = menu();
    for (i, (label, on)) in options.iter().enumerate() {
        item(m, i + 1, label, *on);
    }
    popup(h, m, rtl).checked_sub(1)
}

pub(super) fn pick<V: Copy + PartialEq>(h: HWND, values: &[V], current: V, label: impl Fn(V) -> String) -> Option<V> {
    choose(h, values.iter().map(|&v| (label(v), v == current)).collect()).map(|i| values[i])
}

fn optional_color(h: HWND, off: T, current: Option<u32>, start: u32) -> Option<Option<u32>> {
    let labels = with(|a| [a.t(off), a.t(T::Color)]).unwrap_or_default();
    match choose(h, vec![(labels[0].into(), current.is_none()), (labels[1].into(), current.is_some())])? {
        0 => Some(None),
        _ => choose_color(h, current.unwrap_or(start)).map(Some),
    }
}

fn restyle(f: impl FnOnce(&mut Look)) {
    change(|c| {
        f(&mut c.look);
        c.inherit();
    });
    crate::fence::restyle_all();
}

fn look() -> Look {
    with(|a| a.cfg.look.clone()).unwrap_or_default()
}

fn rule(h: HWND, fence: u64, tab: u64) {
    let Some(options) = with(|a| {
        let t = a.cfg.tab(fence, tab)?;
        Some(Kind::ALL.iter().map(|k| (a.t(i18n::kind(*k)).to_string(), t.kinds.contains(k))).collect())
    })
    .flatten() else {
        return;
    };
    let Some(k) = choose(h, options).map(|i| Kind::ALL[i]) else { return };
    change(|c| c.toggle_rule(fence, tab, k));
}

fn perform(h: HWND, act: rows::Act) {
    use rows::Act::*;
    let l = look();
    match act {
        Color => {
            if let Some(c) = choose_color(h, l.color) {
                restyle(|l| l.color = c);
            }
        }
        Opacity => {
            if let Some(p) = pick(h, &Look::PERCENTS, l.percent(), Look::label) {
                restyle(|l| l.alpha = Look::alpha_for(p));
            }
        }
        IconSize => {
            if let Some(px) = pick(h, &icons::SIZES, l.icon, icons::label) {
                restyle(|l| l.icon = px);
                crate::fence::reload_all();
            }
        }
        Tint => {
            if let Some(t) = optional_color(h, T::None, l.tint, 0xFF_FFFF) {
                restyle(|l| l.tint = t);
            }
        }
        Chameleon => restyle(|l| l.chameleon ^= true),
        Labels => restyle(|l| l.labels ^= true),
        LabelSize => {
            if let Some(s) =
                pick(h, &icons::LabelSize::ALL, l.label_size, |s| with(|a| a.t(i18n::label_size(s)).to_string()).unwrap_or_default())
            {
                restyle(|l| l.label_size = s);
            }
        }
        Spacing => {
            if let Some(s) = pick(h, &icons::Spacing::ALL, l.spacing, |s| with(|a| a.t(i18n::spacing(s)).to_string()).unwrap_or_default()) {
                restyle(|l| l.spacing = s);
            }
        }
        LabelColor => {
            if let Some(c) = optional_color(h, T::Auto, l.label_color, 0xFF_FFFF) {
                restyle(|l| l.label_color = c);
            }
        }
        Browse => restyle(|l| l.browse ^= true),
        Thumbs => {
            restyle(|l| l.thumbs ^= true);
            crate::fence::reload_all();
        }
        Wallpaper => restyle(|l| l.wallpaper ^= true),
        BarHover => restyle(|l| l.bar_hover ^= true),
        TitleAlign => {
            if let Some(m) = pick(h, &grid::Align::ALL, l.title_align, |m| with(|a| a.t(i18n::align(m)).to_string()).unwrap_or_default()) {
                restyle(|l| l.title_align = m);
            }
        }
        TitleSize => {
            if let Some(z) =
                pick(h, &icons::LabelSize::ALL, l.title_size, |z| with(|a| a.t(i18n::label_size(z)).to_string()).unwrap_or_default())
            {
                restyle(|l| l.title_size = z);
            }
        }
        BarColor => {
            if let Some(c) = optional_color(h, T::Auto, l.bar_color, l.color) {
                restyle(|l| l.bar_color = c);
            }
        }
        Align => {
            if let Some(m) = pick(h, &grid::Align::ALL, l.align, |m| with(|a| a.t(i18n::align(m)).to_string()).unwrap_or_default()) {
                restyle(|l| l.align = m);
            }
        }
        AutoHeight => restyle(|l| l.auto_height ^= true),
        Speed => {
            let current = with(|a| a.cfg.roll_ms).unwrap_or(anim::DEFAULT_MS);
            if let Some(ms) = pick(h, &anim::SPEEDS, current, |ms| with(|a| rows::speed_label(a, ms)).unwrap_or_default()) {
                change(|c| c.roll_ms = ms);
            }
        }
        ApplyAll => {
            if crate::report::confirm(h, T::ConfirmApplyAll) {
                change(|c| c.reset_look(None));
                crate::fence::restyle_all();
            }
        }
        KeepOnDesktop => keep::toggle(),
        DblClick => {
            change(|c| c.dblclick_hide ^= true);
            visibility::sync_dblclick();
            visibility::reveal();
        }
        ResetKey(s) => {
            visibility::hotkeys::set(s, s.default_key());
        }
        Key(s) => {
            visibility::hotkeys::suspend();
            with(|a| a.panel.as_mut().map(|p| p.capture = Some(s)));
        }
        Screens => {
            let current = with(|a| a.cfg.screen_adapt).unwrap_or_default();
            if let Some(m) = pick(h, &Adapt::ALL, current, |m| with(|a| rows::adapt_label(a, m).to_string()).unwrap_or_default()) {
                change(|c| c.screen_adapt = m);
            }
        }
        AutoSort => change(|c| c.auto_sort ^= true),
        Rule(fence, tab) => rule(h, fence, tab),
        SortNow => rules::run(false, true),
        NameRule(i) => super::rules::name(h, i),
        AddNameRule => super::rules::add_name(),
        AgeRule(f, t) => super::rules::age(h, f, t),
        AddAgeRule => super::rules::add_age(h),
        Export => crate::backup::export(h),
        Import => crate::backup::import(h),
        Lang => {
            let rtl = with(|a| a.rtl()).unwrap_or(false);
            if let Some(m) = with(|a| prefs::lang_menu(a)) {
                prefs::set_lang(popup(h, m, rtl));
            }
        }
        Autostart => prefs::toggle_autostart(),
        AutoUpdate => prefs::toggle_auto_update(),
        Check => {
            if let Some(t) = tray::hwnd() {
                updates::check_now(t);
            }
        }
    }
}

pub(super) fn click(h: HWND, i: usize, offset: i32) {
    let pick = |r: &rows::Row| r.reset.filter(|_| rows::on_reset(r, offset)).or(r.act);
    if let Some(act) = with(|a| rows::build(a, a.panel.as_ref().map_or(0, |p| p.page)).get(i).and_then(pick)).flatten() {
        crate::undo::step(|| perform(h, act));
        refresh();
    }
}
