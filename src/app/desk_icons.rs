use super::{change, with};
use crate::{
    domain::{Config, system::Sys},
    shell::system::{set_visibility, visibility},
};

const HIDDEN: u32 = 1;

fn before(s: Sys) -> Option<u32> {
    let now = visibility(s);
    if s == Sys::RecycleBin && now == Some(HIDDEN) { None } else { now }
}

pub fn hide(s: Sys) {
    if with(|a| a.cfg.desk_icons.iter().any(|(x, _)| *x == s)) == Some(false) {
        let prev = before(s);
        change(|c| c.desk_icons.push((s, prev)));
    }
    set_visibility(s, Some(HIDDEN));
}

fn forget(s: Sys) -> Option<Option<u32>> {
    let mut out = None;
    change(|c| {
        out = c.desk_icons.iter().position(|(x, _)| *x == s).map(|i| c.desk_icons.remove(i).1);
    });
    out
}

pub fn restore(s: Sys) {
    let prev = forget(s).unwrap_or(None);
    set_visibility(s, prev);
}

pub fn show(s: Sys) {
    forget(s);
    set_visibility(s, Some(0));
}

pub fn hide_placed() {
    with(|a| a.cfg.placed()).unwrap_or_default().into_iter().for_each(hide);
}

pub fn restore_all(cfg: &Config) {
    for s in cfg.placed() {
        let prev = cfg.desk_icons.iter().find(|(x, _)| *x == s).and_then(|(_, p)| *p);
        set_visibility(s, prev);
    }
}
