use std::path::Path;
use windows::Win32::{
    Foundation::{HANDLE, HGLOBAL},
    System::{
        DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
        Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock},
        Ole::CF_HDROP,
    },
    UI::Shell::DROPFILES,
};

pub fn set_files(paths: &[&Path]) -> bool {
    let mut wide: Vec<u16> =
        paths.iter().flat_map(|p| p.as_os_str().to_string_lossy().encode_utf16().chain([0]).collect::<Vec<_>>()).collect();
    wide.push(0);
    let head = size_of::<DROPFILES>();
    unsafe {
        let Ok(mem) = GlobalAlloc(GMEM_MOVEABLE, head + wide.len() * 2) else { return false };
        let p = GlobalLock(mem) as *mut u8;
        *(p as *mut DROPFILES) = DROPFILES { pFiles: head as u32, fWide: true.into(), ..Default::default() };
        std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, p.add(head), wide.len() * 2);
        let _ = GlobalUnlock(mem);
        if OpenClipboard(None).is_err() {
            return false;
        }
        let _ = EmptyClipboard();
        let ok = SetClipboardData(CF_HDROP.0 as u32, Some(HANDLE(mem.0))).is_ok();
        let _ = CloseClipboard();
        let _: HGLOBAL = mem;
        ok
    }
}
