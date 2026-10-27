use crate::win::wide;
use std::env;
use windows::{Win32::System::Registry::*, core::PCWSTR};

const RUN: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const NAME: &str = "RustDesktopIcons";
const VERB: &str = "Software\\Classes\\DesktopBackground\\Shell\\RustDesktopIcons";
pub const NEW_ARG: &str = "--new";
pub const UNINSTALL_ARG: &str = "--uninstall";

fn exe() -> Option<String> {
    env::current_exe().ok().map(|e| e.display().to_string())
}

pub(super) fn reg_write(key: &str, name: &str, kind: REG_VALUE_TYPE, data: &[u8]) -> bool {
    let (k, n) = (wide(key), wide(name));
    unsafe {
        RegSetKeyValueW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()), PCWSTR(n.as_ptr()), kind.0, Some(data.as_ptr().cast()), data.len() as u32)
    }
    .is_ok()
}

pub(super) fn reg_delete(key: &str, name: &str) -> bool {
    let (k, n) = (wide(key), wide(name));
    unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()), PCWSTR(n.as_ptr())) }.is_ok()
}

pub(super) fn reg_dword(key: &str, name: &str) -> Option<u32> {
    let (k, n) = (wide(key), wide(name));
    let (mut v, mut size) = (0u32, 4u32);
    let flags = RRF_RT_REG_DWORD;
    unsafe {
        RegGetValueW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()), PCWSTR(n.as_ptr()), flags, None, Some(&mut v as *mut u32 as _), Some(&mut size))
    }
    .is_ok()
    .then_some(v)
}

fn reg_set(key: &str, name: &str, value: &str) {
    let bytes: Vec<u8> = wide(value).iter().flat_map(|c| c.to_le_bytes()).collect();
    reg_write(key, name, REG_SZ, &bytes);
}

pub fn set_autostart(on: bool) {
    if cfg!(debug_assertions) {
        return;
    }
    if !on {
        reg_delete(RUN, NAME);
    } else if let Some(exe) = exe() {
        reg_set(RUN, NAME, &format!("\"{exe}\""));
    }
}

pub fn set_desktop_verb(label: &str) {
    let Some(exe) = exe() else { return };
    reg_set(VERB, "MUIVerb", label);
    reg_set(VERB, "Icon", &format!("\"{exe}\",0"));
    reg_set(&format!("{VERB}\\command"), "", &format!("\"{exe}\" {NEW_ARG}"));
}
