use super::{fence_of, update, view};
use crate::domain::{Look, inherit::Field};
use crate::{i18n::T, keep, report, rules, shell, store, win::*};
use std::path::Path;
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};

pub fn evacuate(dir: &Path) -> Vec<String> {
    let Some(desk) = shell::desktop() else { return vec![String::from("Desktop")] };
    let (moved, errors) = store::evacuate(dir, &desk);
    moved.into_iter().for_each(rules::ignore);
    errors
}

fn recolor(h: HWND, field: Field, start: impl Fn(&Look) -> u32, put: impl FnOnce(&mut Look, u32)) {
    if let Some(c) = fence_of(h).and_then(|f| choose_color(h, start(&f.look))) {
        update(h, |f| f.set(field, |l| put(l, c)));
    }
}

pub(super) fn color(h: HWND) {
    recolor(h, Field::Color, |l| l.color, |l, c| l.color = c);
}

pub(super) fn tint(h: HWND) {
    recolor(h, Field::Tint, |l| l.tint.unwrap_or(0xFF_FFFF), |l, c| l.tint = Some(c));
}

pub(super) fn bar_color(h: HWND) {
    recolor(h, Field::BarColor, |l| l.bar_color.unwrap_or(l.color), |l, c| l.bar_color = Some(c));
}

pub(super) fn label_color(h: HWND) {
    recolor(h, Field::LabelColor, |l| l.label_color.unwrap_or(0xFF_FFFF), |l, c| l.label_color = Some(c));
}

pub(super) fn reset_look(h: HWND) {
    let Some(id) = super::read(h, |f| f.id) else { return };
    crate::app::edit(|a| a.cfg.reset_look(Some(id)));
    super::restyle(h);
}

pub(super) fn delete(h: HWND) {
    let Some(f) = fence_of(h) else { return };
    if !crate::report::confirm(h, T::ConfirmDelete) {
        return;
    }
    view(h, |v| v.watches.clear());
    let errors: Vec<String> = f.tabs.iter().filter(|t| t.portal.is_none()).flat_map(|t| evacuate(&store::tab_dir(t))).collect();
    if !errors.is_empty() {
        report::failures(T::ErrDelete, &errors);
        return super::bind(h);
    }
    crate::app::edit(|a| {
        a.views.retain(|v| v.hwnd != h);
        a.cfg.fences.retain(|x| x.id != f.id);
    });
    keep::soon();
    super::system::released(&f.tabs);
    unsafe { DestroyWindow(h).ok() };
}
