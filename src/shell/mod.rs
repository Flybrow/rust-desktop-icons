pub mod clip;
mod data;
pub mod details;
mod dnd;
pub mod hide;
mod icons;
mod link;
pub mod menu;
mod ops;
pub mod pick;
mod reg;
pub mod system;
pub mod thumb;
pub mod vdesk;
pub mod vwatch;
mod watch;

use crate::win::wide_path;
pub use data::files;
pub use dnd::{DragImage, drag_out, drop_target};
pub use icons::draw_icon;
pub use link::{folder_target, link_into, link_target};
pub use ops::{delete, transfer, transfer_to};
pub use reg::{NEW_ARG, PORTAL_ARG, RELAUNCH_ARG, UNINSTALL_ARG, light_theme, machine, set_autostart, set_desktop_verbs};
use std::path::{Path, PathBuf};
pub use watch::{Watch, notify_moved};
use windows::{
    Win32::{
        Foundation::CloseHandle,
        Globalization::GetUserDefaultLocaleName,
        Storage::FileSystem::*,
        System::Com::CoTaskMemFree,
        UI::{
            Shell::*,
            WindowsAndMessaging::{ASFW_ANY, AllowSetForegroundWindow, SW_SHOWNORMAL},
        },
    },
    core::*,
};

pub fn locale() -> &'static str {
    static LOCALE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    LOCALE.get_or_init(|| {
        let mut b = [0u16; 85];
        let n = unsafe { GetUserDefaultLocaleName(&mut b) } as usize;
        String::from_utf16_lossy(&b[..n.saturating_sub(1)])
    })
}

fn known(id: &GUID) -> Option<PathBuf> {
    unsafe {
        let p = SHGetKnownFolderPath(id, KNOWN_FOLDER_FLAG(0), None).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as _));
        s.map(PathBuf::from)
    }
}

pub fn file_id(p: &Path) -> u64 {
    let w = wide_path(p);
    unsafe {
        let share = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;
        let Ok(h) = CreateFileW(PCWSTR(w.as_ptr()), 0, share, None, OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS, None) else { return 0 };
        let mut i = BY_HANDLE_FILE_INFORMATION::default();
        let ok = GetFileInformationByHandle(h, &mut i).is_ok();
        let _ = CloseHandle(h);
        if ok { (i.nFileIndexHigh as u64) << 32 | i.nFileIndexLow as u64 } else { 0 }
    }
}

pub fn desktop() -> Option<PathBuf> {
    known(&FOLDERID_Desktop)
}

pub fn desktops() -> Vec<PathBuf> {
    [known(&FOLDERID_Desktop), known(&FOLDERID_PublicDesktop)].into_iter().flatten().collect()
}

pub fn on_desktop(p: &Path) -> Option<usize> {
    desktops().iter().position(|d| p.parent().is_some_and(|q| same_path(q, d)))
}

pub fn same_path(a: &Path, b: &Path) -> bool {
    let text = |p: &Path| p.to_string_lossy().trim_end_matches('\\').to_lowercase();
    text(a) == text(b) || matches!((std::fs::canonicalize(a), std::fs::canonicalize(b)), (Ok(x), Ok(y)) if text(&x) == text(&y))
}

pub fn info(p: &Path) -> (i32, Vec<u16>) {
    let mut i = SHFILEINFOW::default();
    let w = wide_path(p);
    let flags = SHGFI_SYSICONINDEX | SHGFI_DISPLAYNAME;
    unsafe { SHGetFileInfoW(PCWSTR(w.as_ptr()), FILE_FLAGS_AND_ATTRIBUTES(0), Some(&mut i), size_of::<SHFILEINFOW>() as u32, flags) };
    (i.iIcon, crate::win::until_nul(&i.szDisplayName).to_vec())
}

pub fn open(p: &Path) {
    let sys = crate::domain::system::Sys::of(p);
    let w = sys.map_or_else(|| wide_path(p), |s| wide_path(Path::new(s.folder())));
    let dir = p.parent().filter(|_| sys.is_none() && p.is_file()).map(wide_path);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_FLAG_LOG_USAGE,
        lpFile: PCWSTR(w.as_ptr()),
        lpDirectory: dir.as_ref().map_or(PCWSTR::null(), |d| PCWSTR(d.as_ptr())),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    let _ = unsafe { AllowSetForegroundWindow(ASFW_ANY) };
    let _ = unsafe { ShellExecuteExW(&mut info) };
}
