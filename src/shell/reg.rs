use crate::win::wide;
use std::env;
use windows::{Win32::System::Registry::*, core::PCWSTR};

const RUN: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const NAME: &str = "RustDesktopIcons";
const VERB: &str = "Software\\Classes\\DesktopBackground\\Shell\\RustDesktopIcons";
pub const NEW_ARG: &str = "--new";
pub const PORTAL_ARG: &str = "--portal";
pub const UNINSTALL_ARG: &str = "--uninstall";
pub const RELAUNCH_ARG: &str = "--relaunch";

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

fn reg_bytes_in(root: HKEY, key: &str, name: &str, flags: REG_ROUTINE_FLAGS) -> Option<Vec<u8>> {
    let (k, n) = (wide(key), wide(name));
    let (k, n) = (PCWSTR(k.as_ptr()), PCWSTR(n.as_ptr()));
    let mut size = 0u32;
    unsafe { RegGetValueW(root, k, n, flags, None, None, Some(&mut size)) }.ok().ok()?;
    let mut buf = vec![0u8; size as usize];
    unsafe { RegGetValueW(root, k, n, flags, None, Some(buf.as_mut_ptr().cast()), Some(&mut size)) }.ok().ok()?;
    buf.truncate(size as usize);
    Some(buf)
}

pub(super) fn reg_bytes(key: &str, name: &str, flags: REG_ROUTINE_FLAGS) -> Option<Vec<u8>> {
    reg_bytes_in(HKEY_CURRENT_USER, key, name, flags)
}

pub fn machine() -> Option<String> {
    let b = reg_bytes_in(HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Cryptography", "MachineGuid", RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY)?;
    crate::domain::groups::utf16z(&b).map(|g| crate::domain::backup::machine_id(&g))
}

pub(super) fn reg_dword(key: &str, name: &str) -> Option<u32> {
    reg_bytes(key, name, RRF_RT_REG_DWORD)?.try_into().ok().map(u32::from_le_bytes)
}

pub fn light_theme() -> bool {
    reg_dword(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "AppsUseLightTheme") == Some(1)
}

fn reg_set(key: &str, name: &str, value: &str) {
    let bytes: Vec<u8> = wide(value).iter().flat_map(|c| c.to_le_bytes()).collect();
    reg_write(key, name, REG_SZ, &bytes);
}

pub fn set_autostart(on: bool) -> bool {
    if cfg!(debug_assertions) {
        return true;
    }
    if !on {
        reg_delete(RUN, NAME);
    } else if let Some(exe) = exe() {
        reg_set(RUN, NAME, &format!("\"{exe}\""));
    }
    reg_bytes(RUN, NAME, RRF_RT_REG_SZ).is_some() == on
}

pub fn set_desktop_verbs(fence: &str, portal: &str) {
    let Some(exe) = exe() else { return };
    for (key, label, arg) in [(VERB.to_string(), fence, NEW_ARG), (format!("{VERB}.Portal"), portal, PORTAL_ARG)] {
        reg_set(&key, "MUIVerb", label);
        reg_set(&key, "Icon", &format!("\"{exe}\",0"));
        reg_set(&format!("{key}\\command"), "", &format!("\"{exe}\" {arg}"));
    }
}
