use super::reg::{reg_delete, reg_dword, reg_write};
use crate::{domain::system::Sys, win::wide};
use std::path::Path;
use windows::{
    Win32::{
        Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
        System::{
            Com::{CoTaskMemFree, IDataObject},
            Registry::REG_DWORD,
        },
        UI::Shell::{Common::ITEMIDLIST, *},
    },
    core::PCWSTR,
};

const HIDE: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\HideDesktopIcons\\NewStartPanel";

pub fn info(s: Sys) -> (i32, Vec<u16>) {
    let mut i = SHFILEINFOW::default();
    let name = wide(&format!("::{}", s.clsid()));
    unsafe {
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        if SHParseDisplayName(PCWSTR(name.as_ptr()), None, &mut pidl, 0, None).is_ok() && !pidl.is_null() {
            let flags = SHGFI_PIDL | SHGFI_SYSICONINDEX | SHGFI_DISPLAYNAME;
            SHGetFileInfoW(PCWSTR(pidl as *const u16), FILE_FLAGS_AND_ATTRIBUTES(0), Some(&mut i), size_of::<SHFILEINFOW>() as u32, flags);
            CoTaskMemFree(Some(pidl as _));
        }
    }
    let n = i.szDisplayName.iter().position(|&c| c == 0).unwrap_or(0);
    (i.iIcon, i.szDisplayName[..n].to_vec())
}

pub fn name(s: Sys) -> String {
    String::from_utf16_lossy(&info(s).1)
}

pub fn in_data(data: &IDataObject) -> Vec<Sys> {
    unsafe {
        let Ok(items) = SHCreateShellItemArrayFromDataObject::<_, IShellItemArray>(data) else { return vec![] };
        (0..items.GetCount().unwrap_or(0))
            .filter_map(|i| {
                let name = items.GetItemAt(i).and_then(|it| it.GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING)).ok()?;
                let s = name.to_string().ok().and_then(|n| Sys::of(Path::new(&n)));
                CoTaskMemFree(Some(name.0 as _));
                s
            })
            .collect()
    }
}

pub fn visibility(s: Sys) -> Option<u32> {
    reg_dword(HIDE, s.clsid())
}

pub fn set_visibility(s: Sys, value: Option<u32>) {
    if cfg!(debug_assertions) {
        return;
    }
    let ok = match value {
        Some(v) => reg_write(HIDE, s.clsid(), REG_DWORD, &v.to_le_bytes()),
        None => reg_delete(HIDE, s.clsid()) || visibility(s).is_none(),
    };
    if !ok {
        crate::report::log(&format!("cannot change the desktop visibility of {}", s.clsid()));
    }
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
}
