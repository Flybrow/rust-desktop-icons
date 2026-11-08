use std::path::PathBuf;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Com::*;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Shell::Common::{ITEMIDLIST, STRRET};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

pub const MASK: u32 = (FWF_AUTOARRANGE.0 | FWF_SNAPTOGRID.0) as u32;

pub fn desktop() -> PathBuf {
    unsafe { PathBuf::from(SHGetKnownFolderPath(&FOLDERID_Desktop, KNOWN_FOLDER_FLAG(0), None).unwrap().to_string().unwrap()) }
}

pub fn view() -> Result<IFolderView2> {
    unsafe {
        let windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL)?;
        let mut hwnd = 0;
        let disp =
            windows.FindWindowSW(&VARIANT::from(CSIDL_DESKTOP as i32), &VARIANT::default(), SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH)?;
        let browser: IShellBrowser = disp.cast::<IServiceProvider>()?.QueryService(&SID_STopLevelBrowser)?;
        browser.QueryActiveShellView()?.cast()
    }
}

pub fn positions() -> Vec<(String, POINT)> {
    let Ok(v) = view() else { return vec![] };
    let mut out = vec![];
    unsafe {
        let (Ok(folder), Ok(list)) = (v.GetFolder::<IShellFolder>(), v.Items::<IEnumIDList>(SVGIO_ALLVIEW)) else { return out };
        loop {
            let mut pidl: [*mut ITEMIDLIST; 1] = [std::ptr::null_mut()];
            if list.Next(&mut pidl, None).0 != 0 {
                break;
            }
            let mut sr = STRRET::default();
            let mut s = PWSTR::null();
            if folder.GetDisplayNameOf(pidl[0], SHGDN_FORPARSING, &mut sr).is_ok() && StrRetToStrW(&mut sr, Some(pidl[0]), &mut s).is_ok() {
                out.push((s.to_string().unwrap_or_default(), v.GetItemPosition(pidl[0]).unwrap_or_default()));
                CoTaskMemFree(Some(s.0 as _));
            }
            CoTaskMemFree(Some(pidl[0] as _));
        }
    }
    out
}

pub fn listview() -> HWND {
    unsafe {
        let progman = FindWindowW(w!("Progman"), None).unwrap_or_default();
        let mut view = FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None).ok();
        let mut worker = None;
        while view.is_none() {
            worker = FindWindowExW(None, worker, w!("WorkerW"), None).ok();
            if worker.is_none() {
                break;
            }
            view = FindWindowExW(worker, None, w!("SHELLDLL_DefView"), None).ok();
        }
        FindWindowExW(view, None, w!("SysListView32"), None).unwrap_or_default()
    }
}

pub fn screen(pt: POINT) -> (i32, i32) {
    let mut p = pt;
    let _ = unsafe { ClientToScreen(listview(), &mut p) };
    (p.x, p.y)
}

pub fn flags() -> u32 {
    view().and_then(|v| unsafe { v.GetCurrentFolderFlags() }).unwrap_or(0) & MASK
}

static APP: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

pub fn cover() -> Option<RECT> {
    let pid = APP.load(std::sync::atomic::Ordering::Relaxed);
    super::windows(pid, "RustDesktopIcons.Cover").first().copied().map(super::rect)
}

pub fn stacked(pt: POINT) -> bool {
    let (x, y) = screen(pt);
    cover().is_some_and(|r| (r.left..r.right).contains(&x) && (r.top..r.bottom).contains(&y))
}

pub fn hidden(names: &[String]) -> usize {
    positions()
        .iter()
        .filter(|(p, pt)| stacked(*pt) && names.iter().any(|n| p.to_lowercase().ends_with(&format!("\\{}", n.to_lowercase()))))
        .count()
}

pub fn follow(pid: u32) {
    APP.store(pid, std::sync::atomic::Ordering::Relaxed);
}
