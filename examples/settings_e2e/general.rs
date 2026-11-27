use super::{DESCRIBED, HEADER, Ui};
use crate::support::{self, fences, poll, wait};
use serde_json::{Value, json};
use std::{fs, path::Path};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_RETURN;

pub fn general(u: &mut Ui, export: &Path) {
    let general = [HEADER, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED];
    u.page(8);
    u.row(&general, 1);
    u.pick(&["Français"]);
    let french = u.until(|c| c["lang"] == "fr");
    u.row(&general, 1);
    u.pick(&["English"]);
    u.check(french && u.until(|c| c["lang"] == "en"), "General → Language switches and back");
    u.row(&general, 3);
    u.check(u.until(|c| c["auto_update"] == true), "General → Automatic updates toggles");
    u.row(&general, 3);
    u.row(&general, 2);
    let off = u.until(|c| c["autostart"] == false);
    u.row(&general, 2);
    u.check(off && u.until(|c| c["autostart"] == true), "General → Start with Windows toggles");
    u.row(&general, 5);
    let exported: Value = poll(3, || fs::read(export).ok().and_then(|b| serde_json::from_slice(&b).ok())).unwrap_or_default();
    u.check(
        exported["fences"].as_array().is_some_and(|a| a.len() == 3) && exported["look"]["spacing"] == "Wide",
        "Export writes the whole configuration",
    );
    let mut edited = exported.clone();
    edited["fences"].as_array_mut().unwrap().retain(|f| f["id"] != 2);
    edited["dblclick_hide"] = json!(true);
    edited["auto_update"] = json!(true);
    fs::write(export, serde_json::to_vec(&edited).unwrap()).unwrap();
    let auto_update = u.cfg()["auto_update"].clone();
    u.row(&general, 6);
    let asked = support::answer(u.pid, &[VK_RETURN]);
    u.check(
        asked && u.until(|c| c["fences"].as_array().is_some_and(|a| a.len() == 2) && c["dblclick_hide"] == true),
        "Import asks, then replaces fences and settings",
    );
    u.check(u.cfg()["auto_update"] == auto_update, "Import keeps this PC's own settings");
    u.check(wait(3, || fences(u.pid).len() == 2), "the imported fences are shown");
    fs::write(export, b"{ not json").unwrap();
    u.row(&general, 6);
    let warned = support::answer(u.pid, &[VK_RETURN]);
    u.check(warned && u.cfg()["fences"].as_array().is_some_and(|a| a.len() == 2), "a broken file is reported and changes nothing");
}
