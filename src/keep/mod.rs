pub mod cover;
mod keeper;
mod migrate;
mod place;

use crate::{
    app::{change, with},
    domain::pins::{Pin, reconcile as follow},
    fence, shell,
};
pub use keeper::{SOON, TICK, apply, restore, soon};
pub use migrate::{WM_KEEP, finish, sync};
pub use place::{paste, place, release};
use std::{fs, path::PathBuf};

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
}

pub fn reconcile() {
    let Some(pins) = with(|a| a.cfg.fences.iter().flat_map(|f| f.tabs.iter().map(|t| (f.id, t.id, t.pins.clone()))).collect::<Vec<_>>())
    else {
        return;
    };
    if pins.iter().all(|(_, _, p)| p.iter().all(|p| p.path.exists())) {
        return;
    }
    let entries: Vec<(PathBuf, u64)> = shell::desktops()
        .iter()
        .flat_map(|d| fs::read_dir(d).into_iter().flatten().flatten())
        .map(|e| e.path())
        .map(|p| (shell::file_id(&p), p))
        .map(|(id, p)| (p, id))
        .collect();
    change(|c| {
        for (f, t, p) in &pins {
            if let Some(tab) = c.tab_mut(*f, *t) {
                tab.pins = follow(p, &entries);
            }
        }
    });
    fence::reload_all();
}
