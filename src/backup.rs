use crate::{
    app::{self, desk_icons, edit, with},
    domain::{
        Config,
        backup::{self, Imported},
    },
    i18n::T,
    report, shell, store, visibility,
};
use std::{fs, io, path::Path};
use windows::Win32::Foundation::HWND;

fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    let mut f = fs::File::create(&tmp)?;
    io::Write::write_all(&mut f, bytes)?;
    f.sync_all()?;
    drop(f);
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

pub fn export(h: HWND) {
    let Some(path) = shell::pick::save_json(h, "rust-desktop-icons.json") else { return };
    let machine = shell::machine();
    let json = with(|a| backup::export(&a.cfg, machine.as_deref()).map_err(|e| e.to_string())).unwrap_or_else(|| Err("busy".into()));
    if let Err(e) = json.and_then(|j| write_new(&path, &j).map_err(|e| e.to_string())) {
        report::alert(T::ErrSave, &format!("{} ({e})", path.display()));
    }
}

pub fn import(h: HWND) {
    let Some(path) = shell::pick::open_json(h) else { return };
    let parsed = fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|b| Config::parse(&b).map(|c| (c, backup::machine_of(&b))).map_err(|e| e.to_string()));
    let (other, from) = match parsed {
        Ok(c) => c,
        Err(e) => return report::alert(T::ErrImport, &e),
    };
    if !report::confirm(h, T::ConfirmImport) {
        return;
    }
    let foreign = backup::foreign(from.as_deref(), shell::machine().as_deref());
    let dirs = if foreign { store::dir_ids() } else { vec![] };
    crate::undo::step(|| adopt(|c| c.import(other, foreign, &dirs)));
}

pub fn adopt(make: impl FnOnce(&Config) -> Imported) {
    let Some(Imported { orphans, gone, foreign, .. }) = edit(|a| {
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
    if foreign {
        app::recover();
    }
    gone.into_iter().for_each(desk_icons::restore);
    app::rebuild();
    desk_icons::hide_placed();
    visibility::hotkeys::register();
    visibility::sync_dblclick();
    app::register_verb();
    report::failures(T::ErrMove, &errors);
}
