pub mod cover;
mod dll;
mod inject;
mod keeper;
mod migrate;
mod place;

use crate::{
    app::{change, with},
    domain::pins::{GRACE_MS, Listing, Missing, Pin},
    fence, rules, shell,
    win::now_ms,
};
pub use inject::reply;
pub use keeper::{SOON, TICK, apply, restore, soon};
pub use migrate::{WM_KEEP, finish, sync};
pub use place::{paste, place, release};
use std::{cell::RefCell, fs, path::PathBuf};

thread_local! {
    static MISSING: RefCell<Missing> = RefCell::new(Missing::default());
}

pub fn live() -> bool {
    !cfg!(debug_assertions) || std::env::var_os("RDI_REAL_DESKTOP").is_some()
}

pub fn on() -> bool {
    with(|a| a.cfg.keep_on_desktop).unwrap_or(false)
}

pub fn pin_of(path: PathBuf) -> Pin {
    Pin { id: shell::file_id(&path), path }
}

pub fn toggle() {
    change(|c| c.keep_on_desktop ^= true);
    sync(true);
    crate::undo::forget();
}

fn listing() -> Option<Listing> {
    shell::desktop()?;
    let mut entries = vec![];
    for d in shell::desktops() {
        for e in fs::read_dir(d).ok()? {
            let p = e.ok()?.path();
            let id = shell::file_id(&p);
            entries.push((p, id));
        }
    }
    Some(Listing::new(entries))
}

pub fn reconcile() {
    let Some(pins) = with(|a| a.cfg.fences.iter().flat_map(|f| f.tabs.iter().map(|t| (f.id, t.id, t.pins.clone()))).collect::<Vec<_>>())
    else {
        return;
    };
    if pins.iter().all(|(_, _, p)| p.iter().all(|p| p.path.exists())) {
        return MISSING.with_borrow_mut(Missing::clear);
    }
    let Some(listing) = listing() else { return };
    let now = now_ms() as u64;
    let mut changed = vec![];
    MISSING.with_borrow_mut(|m| {
        m.prune(pins.iter().flat_map(|(_, _, p)| p));
        for (f, t, p) in &pins {
            let (next, held) = listing.follow(p, m, now);
            held.into_iter().for_each(rules::ignore);
            if &next != p {
                changed.push((*f, *t, next));
            }
        }
        if m.any() {
            keeper::after(GRACE_MS as u32 + 50);
        }
    });
    if changed.is_empty() {
        return;
    }
    change(|c| {
        for (f, t, p) in changed {
            if let Some(tab) = c.tab_mut(f, t) {
                tab.pins = p;
            }
        }
    });
    fence::reload_all();
}
