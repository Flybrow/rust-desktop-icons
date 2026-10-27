pub mod clip;
mod data;
mod dnd;
pub mod hide;
mod link;
pub mod menu;
mod ops;
mod reg;
pub mod system;
mod watch;

use crate::win::wide_path;
pub use data::files;
pub use dnd::{DragImage, drag_out, pick_folder};
pub use link::{link_into, link_target};
pub use ops::{delete, transfer, transfer_to};
pub use reg::{NEW_ARG, UNINSTALL_ARG, set_autostart, set_desktop_verb};
use std::path::{Path, PathBuf};
pub use watch::{Watch, notify_moved};
use windows::{
    Win32::{
        Foundation::COLORREF,
        Foundation::CloseHandle,
        Globalization::GetUserDefaultLocaleName,
        Graphics::Gdi::HDC,
        Storage::FileSystem::*,
        System::Com::CoTaskMemFree,
        UI::{
            Controls::{CLR_NONE, HIMAGELIST, IImageList, ILD_SCALE, ILD_TRANSPARENT, IMAGELISTDRAWPARAMS},
            Shell::*,
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    },
    core::*,
};

pub fn locale() -> String {
    let mut b = [0u16; 85];
    let n = unsafe { GetUserDefaultLocaleName(&mut b) } as usize;
    String::from_utf16_lossy(&b[..n.saturating_sub(1)])
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

fn list(px: i32) -> i32 {
    (match px {
        ..=16 => SHIL_SMALL,
        17..=32 => SHIL_LARGE,
        33..=48 => SHIL_EXTRALARGE,
        _ => SHIL_JUMBO,
    }) as i32
}

pub fn info(p: &Path) -> (i32, Vec<u16>) {
    let mut i = SHFILEINFOW::default();
    let w = wide_path(p);
    let flags = SHGFI_SYSICONINDEX | SHGFI_DISPLAYNAME;
    unsafe { SHGetFileInfoW(PCWSTR(w.as_ptr()), FILE_FLAGS_AND_ATTRIBUTES(0), Some(&mut i), size_of::<SHFILEINFOW>() as u32, flags) };
    let n = i.szDisplayName.iter().position(|&c| c == 0).unwrap_or(i.szDisplayName.len());
    (i.iIcon, i.szDisplayName[..n].to_vec())
}

pub fn draw_icon(dc: HDC, index: i32, (x, y): (i32, i32), px: i32) {
    unsafe {
        if let Ok(l) = SHGetImageList::<IImageList>(list(px)) {
            let p = IMAGELISTDRAWPARAMS {
                cbSize: size_of::<IMAGELISTDRAWPARAMS>() as u32,
                himl: HIMAGELIST(l.as_raw() as _),
                i: index,
                hdcDst: dc,
                x,
                y,
                cx: px,
                cy: px,
                rgbBk: COLORREF(CLR_NONE as u32),
                fStyle: (ILD_TRANSPARENT | ILD_SCALE).0,
                ..Default::default()
            };
            let _ = l.Draw(&p);
        }
    }
}

pub fn open(p: &Path) {
    let w = crate::domain::system::Sys::of(p).map_or_else(|| wide_path(p), |s| wide_path(Path::new(s.folder())));
    unsafe { ShellExecuteW(None, w!("open"), PCWSTR(w.as_ptr()), None, None, SW_SHOWNORMAL) };
}
