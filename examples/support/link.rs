use std::{os::windows::ffi::OsStrExt, path::Path};
use windows::{
    Win32::{
        System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, IPersistFile},
        UI::Shell::{IShellLinkW, ShellLink},
    },
    core::{Interface, PCWSTR},
};

fn wide(p: &Path) -> Vec<u16> {
    p.as_os_str().encode_wide().chain([0]).collect()
}

pub fn shortcut(target: &Path, dst: &Path) {
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
        link.SetPath(PCWSTR(wide(target).as_ptr())).unwrap();
        link.cast::<IPersistFile>().unwrap().Save(PCWSTR(wide(dst).as_ptr()), true).unwrap();
    }
}
