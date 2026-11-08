use super::{drop, fence_of, items};
use crate::{
    app::with,
    domain::{Tab, system::Sys},
    i18n::T,
    keep, report, shell,
    shell::menu::OWN,
    win::*,
};
use std::path::PathBuf;
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::HMENU};

pub(super) enum Target {
    Tab(u64, Box<Tab>),
    Desktop,
}

pub(super) fn submenu(h: HWND) -> (HMENU, Vec<Target>) {
    let here = fence_of(h).map(|f| (f.id, f.active().id));
    let (m, mut targets) = (menu(), vec![]);
    let (tabs, desktop) = with(|a| (a.cfg.move_targets(here), a.t(T::Desktop))).unwrap_or_default();
    for (fence, tab, label) in tabs {
        item(m, OWN as usize + targets.len(), &label, false);
        targets.push(Target::Tab(fence, Box::new(tab)));
    }
    separator(m);
    item(m, OWN as usize + targets.len(), desktop, false);
    targets.push(Target::Desktop);
    (m, targets)
}

pub(super) fn run(h: HWND, paths: Vec<PathBuf>, target: Target) {
    let paths: Vec<PathBuf> = paths.into_iter().filter(|p| Sys::of(p).is_none()).collect();
    match target {
        Target::Tab(fence, tab) => drop::put(h, fence, &tab, paths, true),
        Target::Desktop => {
            let errors = keep::release(&paths, cursor_pos());
            let names: Vec<PathBuf> =
                shell::desktop().map_or(vec![], |d| paths.iter().filter_map(|p| p.file_name()).map(|n| d.join(n)).collect());
            crate::rules::uncover(&names);
            items::reload_all();
            report::failures(T::ErrMove, &errors);
        }
    }
}
