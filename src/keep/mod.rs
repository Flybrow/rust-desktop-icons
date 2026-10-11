mod keeper;
mod place;

use crate::{
    app::{change, rebuild, with},
    domain::{
        Tab,
        pins::{Pin, reconcile as follow},
    },
    fence,
    i18n::T,
    report, rules, shell, store,
};
pub use keeper::{SOON, TICK, apply, release, restore, soon, tick};
pub use place::{paste, place};
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
    sync();
    rebuild();
}

fn tabs() -> Vec<(u64, Tab)> {
    with(|a| a.cfg.fences.iter().flat_map(|f| f.tabs.iter().filter(|t| t.portal.is_none()).map(|t| (f.id, t.clone()))).collect())
        .unwrap_or_default()
}

pub fn sync() {
    if !live() {
        return;
    }
    let (Some(desk), Some(keep)) = (shell::desktop(), with(|a| a.cfg.keep_on_desktop)) else { return };
    let mut errors = vec![];
    for (fence, t) in tabs() {
        let dir = store::tab_dir(&t);
        if keep {
            if fs::read_dir(&dir).is_ok_and(|mut d| d.next().is_some()) {
                let (moved, errs) = store::evacuate(&dir, &desk);
                errors.extend(errs);
                moved.iter().cloned().for_each(rules::ignore);
                change(|c| c.pin(fence, t.id, moved.into_iter().map(pin_of).collect()));
            }
        } else if !t.pins.is_empty() {
            let mut back = vec![];
            for p in &t.pins {
                match store::move_into(&p.path, &dir) {
                    Ok(_) => back.push(p.path.clone()),
                    Err(_) if !p.path.exists() => back.push(p.path.clone()),
                    Err(e) => errors.push(format!("{} ({e})", store::name(&p.path))),
                }
            }
            change(|c| c.unpin(&back));
        }
    }
    if keep {
        apply()
    } else {
        restore()
    }
    fence::reload_all();
    report::failures(T::ErrMove, &errors);
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
