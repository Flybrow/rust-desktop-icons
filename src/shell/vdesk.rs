use super::reg::reg_bytes;
use crate::domain::groups::{guid, guids, utf16z};
use windows::Win32::System::{
    Registry::{RRF_RT_REG_BINARY, RRF_RT_REG_SZ},
    RemoteDesktop::ProcessIdToSessionId,
    Threading::GetCurrentProcessId,
};

const KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\VirtualDesktops";
const SESSIONS: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\SessionInfo";

fn session() -> Option<u32> {
    let mut id = 0;
    unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut id) }.ok().map(|_| id)
}

pub fn keys() -> Vec<String> {
    [Some(KEY.to_string()), session().map(|s| format!(r"{SESSIONS}\{s}\VirtualDesktops"))].into_iter().flatten().collect()
}

pub fn current() -> Option<String> {
    let b = reg_bytes(KEY, "CurrentVirtualDesktop", RRF_RT_REG_BINARY)
        .or_else(|| reg_bytes(&format!(r"{SESSIONS}\{}\VirtualDesktops", session()?), "CurrentVirtualDesktop", RRF_RT_REG_BINARY))?;
    guid(&b)
}

pub fn ids() -> Vec<String> {
    reg_bytes(KEY, "VirtualDesktopIDs", RRF_RT_REG_BINARY).map_or(vec![], |b| guids(&b))
}

pub fn name(id: &str) -> Option<String> {
    utf16z(&reg_bytes(&format!(r"{KEY}\Desktops\{id}"), "Name", RRF_RT_REG_SZ)?)
}
