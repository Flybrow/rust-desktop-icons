use crate::{
    app::{self, desk_icons, edit, with},
    domain::{Config, backup::Imported},
    i18n::T,
    report, shell, store, visibility,
};
use std::fs;
use windows::Win32::Foundation::HWND;

pub fn export(h: HWND) {
    let Some(path) = shell::pick::save_json(h, "rust-desktop-icons.json") else { return };
    let json = with(|a| serde_json::to_vec_pretty(&a.cfg)).unwrap_or_else(|| Ok(vec![]));
    if let Err(e) = json.map_err(|e| e.to_string()).and_then(|j| fs::write(&path, j).map_err(|e| e.to_string())) {
        report::alert(T::ErrSave, &format!("{} ({e})", path.display()));
    }
}

pub fn import(h: HWND) {
    let Some(path) = shell::pick::open_json(h) else { return };
    let parsed = fs::read(&path).map_err(|e| e.to_string()).and_then(|b| Config::parse(&b).map_err(|e| e.to_string()));
    let other = match parsed {
        Ok(c) => c,
        Err(e) => return report::alert(T::ErrImport, &e),
    };
    if !report::confirm(h, T::ConfirmImport) {
        return;
    }
    crate::undo::step(|| adopt(|c| c.import(other)));
}

pub fn adopt(make: impl FnOnce(&Config) -> Imported) {
    let Some(Imported { orphans, gone, .. }) = edit(|a| {
        let mut i = make(&a.cfg);
        a.cfg = std::mem::take(&mut i.cfg);
        i
    }) else {
        return;
    };
    if let Some((auto, updates)) = with(|a| (a.cfg.autostart, a.cfg.auto_update)) {
        shell::set_autostart(auto);
        crate::tray::updates::enable(updates);
    }
    let errors: Vec<String> = orphans.iter().flat_map(|t| crate::fence::evacuate(&store::tab_dir(t))).collect();
    gone.into_iter().for_each(desk_icons::restore);
    app::rebuild();
    desk_icons::hide_placed();
    visibility::hotkeys::register();
    visibility::sync_dblclick();
    app::register_verb();
    report::failures(T::ErrMove, &errors);
}
