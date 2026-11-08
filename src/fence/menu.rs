use super::{actions, arrange, cursor, edit, fence_of, host, items, layout, native, peek, select, system, tabs, update};
use crate::domain::inherit::Field;
use crate::{
    app::with,
    domain::{Look, grid::Align, icons, system::Sys},
    i18n::{self, T},
    keep,
    layered::glyph as g,
    settings, shell, store,
    win::*,
};
use windows::Win32::Foundation::HWND;

const SYS: usize = 90;
const ALIGN: usize = 67;
const ALIGN_END: usize = ALIGN + Align::ALL.len() - 1;
const SYS_END: usize = SYS + Sys::ALL.len() - 1;

pub(super) fn context(h: HWND) {
    let p = cursor(h);
    if let Some(i) = tabs::at(h, p) {
        tabs::select(h, i);
    }
    if let Some(t) = items::item_at(h, p) {
        return native::show(h, &select::focus(h, &t), cursor_pos());
    }
    let Some(f) = fence_of(h) else { return };
    let Some((m, rtl)) = with(|a| {
        let m = menu();
        let (o, sizes, tint, look, behave, tabs, sys) = (menu(), menu(), menu(), menu(), menu(), menu(), menu());
        for (k, &pct) in Look::PERCENTS.iter().enumerate() {
            item(o, 20 + k, &Look::label(pct), f.look.alpha == Look::alpha_for(pct));
        }
        for (k, &px) in icons::SIZES.iter().enumerate() {
            item(sizes, 40 + k, &icons::label(px), f.look.icon == px);
        }
        item(tint, 50, a.t(T::None), f.look.tint.is_none());
        item(tint, 51, a.t(T::Color), f.look.tint.is_some());
        for (k, s) in Sys::ALL.iter().enumerate() {
            item(sys, SYS + k, &shell::system::name(*s), f.active().system.contains(s));
        }
        a.entry(m, 10, T::Rename, false, g::RENAME);
        arrange::submenus(a, m, f.active());
        a.sub(m, T::SystemIcons, sys, g::PC);
        a.entry(m, 13, T::OpenFolder, false, g::FOLDER);
        separator(m);
        a.entry(look, 11, T::Color, false, g::COLOR);
        a.sub(look, T::Opacity, o, g::OPACITY);
        a.sub(look, T::IconSize, sizes, g::SIZE);
        a.sub(look, T::Tint, tint, g::TINT);
        a.entry(look, 65, T::Labels, f.look.labels, g::LABEL);
        let align = menu();
        for (k, al) in Align::ALL.iter().enumerate() {
            item(align, ALIGN + k, a.t(i18n::align(*al)), f.look.align == *al);
        }
        a.sub(look, T::Align, align, g::ALIGN);
        a.entry(look, 16, T::Chameleon, f.look.chameleon, g::EYE);
        if f.customized() {
            separator(look);
            a.entry(look, 66, T::ResetDefault, false, g::REFRESH);
        }
        a.sub(m, T::LookMenu, look, g::COLOR);
        a.entry(behave, 12, T::Roll, f.rolled, g::ROLL);
        a.entry(behave, 18, T::AutoHeight, f.look.auto_height, g::HEIGHT);
        a.entry(behave, 64, T::Vertical, f.vertical, g::TAB);
        a.entry(behave, 19, T::Lock, f.locked, g::LOCK);
        a.sub(m, T::Behavior, behave, g::SETTINGS);
        a.entry(tabs, 60, T::NewTab, false, g::TAB);
        a.entry(tabs, 61, T::NewPortal, false, g::PORTAL);
        if f.tabs.len() > 1 {
            a.entry(tabs, 62, T::DeleteTab, false, g::CLOSE);
        }
        a.sub(m, T::Tabs, tabs, g::TAB);
        separator(m);
        a.entry(m, 15, T::NewFence, false, g::ADD);
        a.entry(m, 14, T::DeleteFence, false, g::DELETE);
        a.entry(m, 17, T::Settings, false, g::SETTINGS);
        (m, a.rtl())
    }) else {
        return;
    };
    let id = popup(host::for_menu(h), m, rtl);
    if arrange::handle(h, id) {
        return;
    }
    match id {
        10 => edit::title(h),
        11 => actions::color(h),
        12 => peek::toggle(h),
        13 if keep::on() && f.active().portal.is_none() => shell::desktop().iter().for_each(|d| shell::open(d)),
        13 => shell::open(&store::tab_dir(f.active())),
        14 => actions::delete(h),
        15 => crate::app::new_fence(),
        16 => {
            update(h, |f| f.set(Field::Chameleon, |l| l.chameleon ^= true));
            super::anim::start(h);
        }
        17 => settings::open(),
        18 => update(h, |f| f.set(Field::AutoHeight, |l| l.auto_height ^= true)),
        65 => update(h, |f| f.set(Field::Labels, |l| l.labels ^= true)),
        66 => actions::reset_look(h),
        k @ ALIGN..=ALIGN_END => update(h, |f| f.set(Field::Align, |l| l.align = Align::ALL[k - ALIGN])),
        19 => update(h, |f| f.locked ^= true),
        k @ 20..=27 => update(h, |f| f.set(Field::Alpha, |l| l.alpha = Look::alpha_for(Look::PERCENTS[k - 20]))),
        k @ 40..=47 => items::set_icon(h, icons::SIZES[k - 40]),
        50 => update(h, |f| f.set(Field::Tint, |l| l.tint = None)),
        51 => actions::tint(h),
        60 => tabs::new_tab(h),
        61 => tabs::new_portal(h),
        62 => tabs::remove(h),
        k @ SYS..=SYS_END => system::toggle(h, Sys::ALL[k - SYS]),
        64 => {
            update(h, |f| f.vertical ^= true);
            layout::orient(h);
            layout::apply(h);
        }
        _ => {}
    }
}
