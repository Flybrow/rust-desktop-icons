use super::data;
use std::path::PathBuf;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
        UI::{Shell::*, WindowsAndMessaging::*},
    },
    core::{Interface, PCSTR, PSTR},
};

const FIRST: u32 = 1;
pub const OWN: u32 = 0x7000;

fn pick(id: u32) -> Pick {
    match id {
        0 => Pick::Nothing,
        id if id >= OWN => Pick::Own((id - OWN) as usize),
        id => Pick::Shell(id - FIRST),
    }
}

#[derive(Debug, PartialEq)]
pub enum Pick {
    Nothing,
    Shell(u32),
    Own(usize),
}
const CMIC_MASK_UNICODE: u32 = 0x4000;

pub struct Native {
    pub cm: IContextMenu,
    menu: HMENU,
}

impl Drop for Native {
    fn drop(&mut self) {
        let _ = unsafe { DestroyMenu(self.menu) };
    }
}

pub fn build(paths: &[PathBuf], extended: bool) -> Option<Native> {
    let cm: IContextMenu = data::items(paths, &BHID_SFUIObject)?;
    let native = Native { cm, menu: unsafe { CreatePopupMenu() }.ok()? };
    let flags = CMF_NORMAL | CMF_CANRENAME | if extended { CMF_EXTENDEDVERBS } else { 0 };
    unsafe { native.cm.QueryContextMenu(native.menu, 0, FIRST, OWN - 1, flags) }.ok().ok()?;
    Some(native)
}

impl Native {
    pub fn prepend(&self, label: &str, sub: HMENU) {
        let text = crate::win::wide(label);
        unsafe {
            let _ = InsertMenuW(self.menu, 0, MF_BYPOSITION | MF_POPUP, sub.0 as usize, windows::core::PCWSTR(text.as_ptr()));
            let _ = InsertMenuW(self.menu, 1, MF_BYPOSITION | MF_SEPARATOR, 0, None);
        }
    }

    pub fn prepend_item(&self, label: &str, own: usize) {
        let text = crate::win::wide(label);
        let _ = unsafe { InsertMenuW(self.menu, 0, MF_BYPOSITION | MF_STRING, OWN as usize + own, windows::core::PCWSTR(text.as_ptr())) };
    }

    pub fn track(&self, h: HWND, (x, y): (i32, i32)) -> Pick {
        let _ = unsafe { SetForegroundWindow(h) };
        pick(unsafe { TrackPopupMenuEx(self.menu, (TPM_RETURNCMD | TPM_RIGHTBUTTON).0, x, y, h, None) }.0 as u32)
    }

    pub fn verb(&self, id: u32) -> String {
        let mut b = [0u16; 64];
        let ok = unsafe { self.cm.GetCommandString(id as usize, GCS_VERBW, None, PSTR(b.as_mut_ptr().cast()), b.len() as u32) };
        ok.map(|_| String::from_utf16_lossy(&b[..b.iter().position(|&c| c == 0).unwrap_or(0)])).unwrap_or_default()
    }

    pub fn invoke(&self, id: u32, h: HWND, (x, y): (i32, i32)) {
        let info = CMINVOKECOMMANDINFOEX {
            cbSize: size_of::<CMINVOKECOMMANDINFOEX>() as u32,
            fMask: CMIC_MASK_UNICODE | CMIC_MASK_PTINVOKE,
            hwnd: h,
            lpVerb: PCSTR(id as usize as *const u8),
            lpVerbW: windows::core::PCWSTR(id as usize as *const u16),
            nShow: SW_SHOWNORMAL.0,
            ptInvoke: POINT { x, y },
            ..Default::default()
        };
        let _ = unsafe { self.cm.InvokeCommand(&info as *const _ as *const CMINVOKECOMMANDINFO) };
    }
}

pub fn forward(cm: &IContextMenu, msg: u32, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
    if let Ok(c3) = cm.cast::<IContextMenu3>() {
        let mut r = LRESULT(0);
        return unsafe { c3.HandleMenuMsg2(msg, wp, lp, Some(&mut r)) }.ok().map(|_| r);
    }
    let c2 = cm.cast::<IContextMenu2>().ok()?;
    unsafe { c2.HandleMenuMsg(msg, wp, lp) }.ok().map(|_| LRESULT(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_ids_split_between_windows_and_the_app() {
        assert_eq!(
            (pick(0), pick(FIRST), pick(OWN - 1), pick(OWN), pick(OWN + 3)),
            (Pick::Nothing, Pick::Shell(0), Pick::Shell(OWN - 2), Pick::Own(0), Pick::Own(3))
        );
    }
}
