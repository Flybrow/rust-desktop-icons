use super::{movable, put};
use crate::{app::with, domain::matching::expired, fence, i18n::T, keep, report, store, tray};
use std::{
    cell::RefCell,
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub const TIMER: usize = tray::timers::AGE;
const FIRST_MS: u32 = 60_000;
const EVERY_MS: u32 = 3_600_000;
const DEPTH: u8 = 4;
const BUDGET: usize = 2000;

thread_local! {
    static FAILED: RefCell<HashSet<PathBuf>> = RefCell::new(HashSet::new());
}

fn every(normal: u32) -> u32 {
    std::env::var("RDI_AGE_MS").ok().filter(|_| cfg!(debug_assertions)).and_then(|v| v.parse().ok()).unwrap_or(normal)
}

pub fn start() {
    tray::timers::set(TIMER, Some(every(FIRST_MS)));
}

fn newest(p: &Path, depth: u8, budget: &mut usize) -> Option<SystemTime> {
    let m = fs::symlink_metadata(p).ok()?;
    let mut t = m.modified().ok()?;
    if m.is_dir() && depth > 0 {
        for e in fs::read_dir(p).ok()? {
            *budget = budget.checked_sub(1)?;
            t = t.max(newest(&e.ok()?.path(), depth - 1, budget)?);
        }
    }
    Some(t)
}

fn age(p: &Path) -> Option<u64> {
    newest(p, DEPTH, &mut { BUDGET })?.elapsed().ok().map(|d| d.as_secs())
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
                Err(x) if FAILED.with(|f| f.borrow_mut().insert(p.clone())) => errors.push(x),
                Err(x) => report::log(&x),
            }
        }
    }
    if moved {
        fence::reload_all();
    }
    report::failures(T::ErrMove, &errors);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::windows::fs::OpenOptionsExt, time::Duration};

    fn aged(p: &Path, days: u64) {
        let f = fs::OpenOptions::new().write(true).custom_flags(0x0200_0000).open(p).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(days * 86_400)).unwrap();
    }

    #[test]
    fn a_folder_is_as_old_as_its_newest_file() {
        let d = std::env::temp_dir().join(format!("rdi-age-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        let deep = d.join("project").join("src");
        fs::create_dir_all(&deep).unwrap();
        fs::write(d.join("old.txt"), "o").unwrap();
        fs::write(deep.join("new.rs"), "n").unwrap();
        aged(&d.join("old.txt"), 90);
        aged(&deep, 90);
        aged(&d.join("project"), 90);
        assert!(age(&d.join("old.txt")).is_some_and(|a| a >= 89 * 86_400));
        assert!(age(&d.join("project")).is_some_and(|a| a < 86_400));
        aged(&deep.join("new.rs"), 60);
        assert!(age(&d.join("project")).is_some_and(|a| (59 * 86_400..61 * 86_400).contains(&a)));
        assert_eq!(newest(&d, 4, &mut 1), None);
        assert!(age(&d.join("missing")).is_none());
        fs::remove_dir_all(&d).unwrap();
    }
}
