use crate::{
    app::{App, change, rebuild, with},
    i18n::T,
    layered::glyph as g,
    win::*,
};
use windows::Win32::UI::WindowsAndMessaging::HMENU;

const SAVE: usize = 20;
const RESTORE: usize = 30;
const FORGET: usize = 50;
const MAX: usize = crate::domain::layouts::SNAPSHOTS;

const RENAME: usize = 70;

pub fn submenu(a: &mut App, m: HMENU) {
    let names: Vec<String> = a.cfg.snapshots.iter().map(|s| s.name.clone()).collect();
    let l = menu();
    a.entry(l, SAVE, T::SaveLayout, false, g::ADD);
    if !names.is_empty() {
        separator(l);
    }
    for (i, n) in names.iter().enumerate().take(MAX) {
        let one = menu();
        a.entry(one, RESTORE + i, T::Restore, false, g::REFRESH);
        a.entry(one, RENAME + i, T::Rename, false, g::RENAME);
        a.entry(one, FORGET + i, T::DeleteLayout, false, g::DELETE);
        crate::win::submenu(l, n, one);
    }
    a.sub(m, T::Layouts, l, g::TAB);
}

pub fn handle(id: usize) -> bool {
    match id {
        SAVE => change(|c| c.snapshot(stamp(false))),
        k if (RESTORE..RESTORE + MAX).contains(&k) => {
            change(|c| {
                c.restore(k - RESTORE);
            });
            rebuild();
        }
        k if (RENAME..RENAME + MAX).contains(&k) => {
            let i = k - RENAME;
            let name = with(|a| a.cfg.snapshots.get(i).map(|s| s.name.clone())).flatten().unwrap_or_default();
            crate::prompt::ask(&name, move |n| change(|c| c.rename_snapshot(i, &n)));
        }
        k if (FORGET..FORGET + MAX).contains(&k) => change(|c| c.forget_snapshot(k - FORGET)),
        _ => return false,
    }
    true
}
