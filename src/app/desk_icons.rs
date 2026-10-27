use super::{change, with};
use crate::{
    domain::{
        Config,
        system::{HIDDEN, Sys},
    },
    shell::system::{set_visibility, visibility},
};

pub fn hide(s: Sys) {
    if with(|a| a.cfg.desk_icons.iter().any(|(x, _)| *x == s)) == Some(false) {
        let prev = s.original(visibility(s));
        change(|c| {
            c.remember(s, prev);
        });
    }
    set_visibility(s, Some(HIDDEN));
}

fn forget(s: Sys) -> Option<Option<u32>> {
    let mut out = None;
    change(|c| out = c.forget(s));
    out
}

pub fn restore(s: Sys) {
    set_visibility(s, forget(s).flatten());
}

pub fn show(s: Sys) {
    forget(s);
    set_visibility(s, Some(0));
}

pub fn hide_placed() {
    with(|a| a.cfg.placed()).unwrap_or_default().into_iter().for_each(hide);
}

pub fn restore_all(cfg: &Config) {
    cfg.placed().into_iter().for_each(|s| set_visibility(s, cfg.original(s)));
}
