use crate::domain::pins::free_slots;
use std::path::{Path, PathBuf};
use windows::{
    Win32::{
        Foundation::{POINT, RECT},
        System::{
            Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree, IServiceProvider},
            Variant::VARIANT,
        },
        UI::{
            Shell::{
                Common::{ITEMIDLIST, STRRET},
                *,
            },
            WindowsAndMessaging::{SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW},
        },
    },
    core::*,
};

pub const MASK: u32 = (FWF_AUTOARRANGE.0 | FWF_SNAPTOGRID.0) as u32;
const AWAY: i32 = -20_000;

struct Entry {
    path: PathBuf,
    pidl: *mut ITEMIDLIST,
    pos: POINT,
}

impl Drop for Entry {
    fn drop(&mut self) {
        unsafe { CoTaskMemFree(Some(self.pidl as _)) };
    }
}

fn away(p: POINT) -> bool {
    p.x <= AWAY / 2
}

fn listed(paths: &[PathBuf], p: &Path) -> bool {
    paths.iter().any(|q| q.as_os_str().eq_ignore_ascii_case(p.as_os_str()))
}

pub struct Desk(IFolderView2);

impl Desk {
    pub fn open() -> Option<Self> {
        unsafe {
            let windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL).ok()?;
            let mut hwnd = 0;
            let loc = VARIANT::from(CSIDL_DESKTOP as i32);
            let disp = windows.FindWindowSW(&loc, &VARIANT::default(), SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH).ok()?;
            let browser: IShellBrowser = disp.cast::<IServiceProvider>().ok()?.QueryService(&SID_STopLevelBrowser).ok()?;
            browser.QueryActiveShellView().ok()?.cast().ok().map(Self)
        }
    }

    fn entries(&self) -> Vec<Entry> {
        let mut out = vec![];
        unsafe {
            let (Ok(folder), Ok(list)) = (self.0.GetFolder::<IShellFolder>(), self.0.Items::<IEnumIDList>(SVGIO_ALLVIEW)) else {
                return out;
            };
            loop {
                let mut pidl = [std::ptr::null_mut()];
                if list.Next(&mut pidl, None).0 != 0 || pidl[0].is_null() {
                    break;
                }
                let pos = self.0.GetItemPosition(pidl[0]).unwrap_or_default();
                let mut sr = STRRET::default();
                let path = folder.GetDisplayNameOf(pidl[0], SHGDN_FORPARSING, &mut sr).ok().and_then(|_| {
                    let mut s = PWSTR::null();
                    StrRetToStrW(&mut sr, Some(pidl[0]), &mut s).ok()?;
                    let text = s.to_string().ok();
                    CoTaskMemFree(Some(s.0 as _));
                    text
                });
                out.push(Entry { path: path.map(PathBuf::from).unwrap_or_default(), pidl: pidl[0], pos });
            }
        }
        out
    }

    pub fn flags(&self) -> Option<u32> {
        unsafe { self.0.GetCurrentFolderFlags() }.ok()
    }

    pub fn set_flags(&self, flags: u32) {
        let _ = unsafe { self.0.SetCurrentFolderFlags(MASK, flags & MASK) };
    }

    fn place(&self, items: &[&Entry], pts: &[POINT]) {
        let pidls: Vec<*const ITEMIDLIST> = items.iter().map(|e| e.pidl as *const _).collect();
        let flags = (SVSI_POSITIONITEM.0 | SVSI_DESELECT.0) as u32;
        let _ = unsafe { self.0.SelectAndPositionItems(pidls.len() as u32, pidls.as_ptr(), Some(pts.as_ptr()), flags) };
    }

    pub fn sync(&self, keep: &[PathBuf]) -> usize {
        let entries = self.entries();
        let hide: Vec<&Entry> = entries.iter().filter(|e| !away(e.pos) && listed(keep, &e.path)).collect();
        let show: Vec<&Entry> = entries.iter().filter(|e| away(e.pos) && !listed(keep, &e.path)).collect();
        if !hide.is_empty() {
            if self.flags().is_some_and(|f| f & MASK != 0) {
                self.set_flags(0);
            }
            let pts: Vec<POINT> = (0..hide.len() as i32).map(|i| POINT { x: AWAY - i % 500 * 16, y: AWAY }).collect();
            self.place(&hide, &pts);
        }
        if !show.is_empty() {
            let taken: Vec<(i32, i32)> = entries.iter().filter(|e| !away(e.pos)).map(|e| (e.pos.x, e.pos.y)).collect();
            let mut cell = POINT::default();
            let _ = unsafe { self.0.GetSpacing(&mut cell) };
            let mut area = RECT::default();
            let _ = unsafe {
                SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut area as *mut _ as _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0))
            };
            let size = (area.right - area.left, area.bottom - area.top);
            let slots = free_slots(&taken, (cell.x.max(75), cell.y.max(75)), size, show.len());
            self.place(&show, &slots.into_iter().map(|(x, y)| POINT { x, y }).collect::<Vec<_>>());
        }
        hide.len() + show.len()
    }
}
