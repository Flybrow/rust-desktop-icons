use super::{movable, put};
use crate::{app::with, domain::matching::expired, fence, i18n::T, keep, report, store, tray};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const TIMER: usize = tray::timers::AGE;
const FIRST_MS: u32 = 60_000;
const EVERY_MS: u32 = 3_600_000;

fn every(normal: u32) -> u32 {
    std::env::var("RDI_AGE_MS").ok().filter(|_| cfg!(debug_assertions)).and_then(|v| v.parse().ok()).unwrap_or(normal)
}

pub fn start() {
    tray::timers::set(TIMER, Some(every(FIRST_MS)));
}

fn age(p: &Path) -> Option<u64> {
    fs::metadata(p).ok()?.modified().ok()?.elapsed().ok().map(|d| d.as_secs())
}

pub fn run() {
    crate::undo::step(expire);
}

fn expire() {
    tray::timers::set(TIMER, Some(every(EVERY_MS)));
    let Some(cfg) = with(|a| a.cfg.clone()) else { return };
    let keep = cfg.keep_on_desktop && keep::live();
    let (mut moved, mut errors) = (false, vec![]);
    for (f, t, e) in cfg.expiring() {
        let Some(tab) = cfg.tab(f, t) else { continue };
        let items: Vec<PathBuf> = if keep {
            tab.pins.iter().map(|p| p.path.clone()).collect()
        } else {
            fs::read_dir(store::tab_dir(tab)).into_iter().flatten().flatten().map(|e| e.path()).collect()
        };
        for p in items.into_iter().filter(|p| movable(p) && age(p).is_some_and(|a| expired(a, e.days))) {
            match put(&cfg, &p, (e.fence, e.tab), keep) {
                Ok(m) => moved |= m,
                Err(x) => errors.push(x),
            }
        }
    }
    if moved {
        fence::reload_all();
    }
    report::failures(T::ErrMove, &errors);
}
