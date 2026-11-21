use super::{edit, with};
use crate::{fence, layered::chrome, settings, shell};
use windows::{Win32::Foundation::LPARAM, core::PCWSTR};

pub fn changed(lp: LPARAM) -> bool {
    lp.0 != 0 && unsafe { PCWSTR(lp.0 as *const u16).to_string() }.is_ok_and(|s| s == "ImmersiveColorSet")
}

pub fn apply(repaint: bool) {
    let Some(theme) = with(|a| a.cfg.theme) else { return };
    let light = theme.light(shell::light_theme());
    let was = chrome::palette().bg;
    chrome::set_light(light);
    if with(|a| a.cfg.retheme(light)) == Some(true) {
        edit(|_| ());
        fence::restyle_all();
    }
    if repaint && was != chrome::palette().bg {
        settings::refresh();
    }
}
