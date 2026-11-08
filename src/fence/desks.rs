use super::update;
use crate::{
    app::App,
    i18n::T,
    layered::glyph as g,
    visibility::desks::{apply, list, sync},
    win::*,
};
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::HMENU};

const ALL: usize = 109;
const ONE: usize = 110;
const ONE_END: usize = ONE + 38;

pub(super) fn submenu(a: &mut App, m: HMENU, desktop: Option<&str>) {
    let desks = list();
    if desks.len() < 2 && desktop.is_none() {
        return;
    }
    let sub = menu();
    item(sub, ALL, a.t(T::AllDesktops), desktop.is_none());
    separator(sub);
    for (k, (id, name)) in desks.iter().take(ONE_END - ONE + 1).enumerate() {
        let label = name.clone().unwrap_or_else(|| a.t(T::DesktopN).replace("{}", &(k + 1).to_string()));
        item(sub, ONE + k, &label, desktop.is_some_and(|d| d.eq_ignore_ascii_case(id)));
    }
    a.sub(m, T::ShowOn, sub, g::PC);
}

pub(super) fn handle(h: HWND, id: usize) -> bool {
    let desktop = match id {
        ALL => None,
        k @ ONE..=ONE_END => match list().into_iter().nth(k - ONE) {
            Some((d, _)) => Some(d),
            None => return true,
        },
        _ => return false,
    };
    update(h, |f| f.desktop = desktop);
    sync();
    apply();
    true
}
