use crate::domain::pins::{corner_slot, free_slots};
use std::path::{Path, PathBuf};
use windows::{
    Win32::{
        Foundation::{HWND, POINT, RECT},
        Graphics::Gdi::{ClientToScreen, ScreenToClient},
        System::{
            Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree, IServiceProvider},
            Variant::VARIANT,
        },
        UI::{
            Shell::{
                Common::{ITEMIDLIST, STRRET},
                *,
            },
            WindowsAndMessaging::{
                FindWindowExW, FindWindowW, GetSystemMetrics, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SPI_GETWORKAREA,
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
            },
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

pub fn listview() -> Option<HWND> {
    unsafe {
        let progman = FindWindowW(w!("Progman"), None).ok()?;
        let mut view = FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None).ok();
        let mut worker = None;
        while view.is_none() {
            worker = Some(FindWindowExW(None, worker, w!("WorkerW"), None).ok()?);
            view = FindWindowExW(worker, None, w!("SHELLDLL_DefView"), None).ok();
        }
        FindWindowExW(view, None, w!("SysListView32"), None).ok()
    }
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

    fn spacing(&self) -> POINT {
        let mut cell = POINT::default();
        let _ = unsafe { self.0.GetSpacing(&mut cell) };
        POINT { x: cell.x.max(75), y: cell.y.max(75) }
    }

    pub fn show_at(&self, paths: &[PathBuf], (x, y): (i32, i32)) {
        let Some(lv) = listview() else { return };
        let mut at = POINT { x, y };
        let _ = unsafe { ScreenToClient(lv, &mut at) };
        let entries = self.entries();
        let todo: Vec<&Entry> = entries.iter().filter(|e| listed(paths, &e.path)).collect();
        let cell = self.spacing();
        let pts: Vec<POINT> = (0..todo.len() as i32).map(|i| POINT { x: at.x - cell.x / 2 + i * cell.x, y: at.y - cell.y / 2 }).collect();
        self.place(&todo, &pts);
    }

    fn area(&self) -> (i32, i32, i32, i32) {
        let mut area = RECT::default();
        let _ =
            unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut area as *mut _ as _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)) };
        let (vx, vy) = unsafe { (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN)) };
        (area.left - vx, area.top - vy, area.right - area.left, area.bottom - area.top)
    }

    pub fn sync(&self, keep: &[PathBuf], prev: Option<(i32, i32)>) -> Option<((i32, i32), RECT)> {
        let entries = self.entries();
        let (cell, area) = (self.spacing(), self.area());
        let near = |p: POINT, (x, y): (i32, i32)| (p.x - x).abs() < cell.x && (p.y - y).abs() < cell.y;
        let others: Vec<(i32, i32)> =
            entries.iter().filter(|e| !away(e.pos) && !listed(keep, &e.path)).map(|e| (e.pos.x, e.pos.y)).collect();
        let inside = |(x, y): (i32, i32)| x >= area.0 && y >= area.1 && x + cell.x <= area.0 + area.2 && y + cell.y <= area.1 + area.3;
        let free = |p: (i32, i32)| inside(p) && !others.iter().any(|&o| near(POINT { x: o.0, y: o.1 }, p));
        let stack = prev.filter(|&p| free(p)).unwrap_or_else(|| corner_slot(&others, (cell.x, cell.y), area));
        let at = |e: &Entry| (e.pos.x, e.pos.y) == stack;
        let hide: Vec<&Entry> = entries.iter().filter(|e| listed(keep, &e.path) && !at(e)).collect();
        let show: Vec<&Entry> = entries.iter().filter(|e| !listed(keep, &e.path) && (away(e.pos) || at(e))).collect();
        if !hide.is_empty() {
            if self.flags().is_some_and(|f| f & MASK != 0) {
                self.set_flags(0);
            }
            self.place(&hide, &vec![POINT { x: stack.0, y: stack.1 }; hide.len()]);
        }
        let picked: Vec<&Entry> = entries
            .iter()
            .filter(|e| at(e) && unsafe { self.0.GetSelectionState(e.pidl) }.is_ok_and(|s| s & SVSI_SELECT.0 as u32 != 0))
            .collect();
        if !picked.is_empty() {
            let pidls: Vec<*const ITEMIDLIST> = picked.iter().map(|e| e.pidl as *const _).collect();
            let _ = unsafe { self.0.SelectAndPositionItems(pidls.len() as u32, pidls.as_ptr(), None, SVSI_DESELECT.0 as u32) };
        }
        if !show.is_empty() {
            let taken: Vec<(i32, i32)> = others.iter().copied().chain([stack]).collect();
            let slots = free_slots(&taken, (cell.x, cell.y), area, show.len());
            self.place(&show, &slots.into_iter().map(|(x, y)| POINT { x, y }).collect::<Vec<_>>());
        }
        let lv = listview()?;
        let mut p = POINT { x: stack.0, y: stack.1 };
        let _ = unsafe { ClientToScreen(lv, &mut p) };
        entries
            .iter()
            .any(|e| listed(keep, &e.path))
            .then_some((stack, RECT { left: p.x - cell.x / 4, top: p.y, right: p.x + cell.x * 5 / 4, bottom: p.y + cell.y }))
    }
}
