use crate::{
    domain::{
        pins::{Wall, clear_spot, covered, same},
        stack::{self, Icon},
    },
    win::work_area,
};
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
                FindWindowExW, GetSystemMetrics, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SPI_GETWORKAREA,
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
            },
        },
    },
    core::*,
};

pub const MASK: u32 = (FWF_AUTOARRANGE.0 | FWF_SNAPTOGRID.0) as u32;
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
    stack::away((p.x, p.y))
}

fn listed(paths: &[PathBuf], p: &Path) -> bool {
    paths.iter().any(|q| same(q, p))
}

pub fn listview() -> Option<HWND> {
    unsafe {
        let progman = crate::win::progman()?;
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
        let pts: Vec<POINT> = stack::row((at.x, at.y), (cell.x, cell.y), todo.len()).into_iter().map(|(x, y)| POINT { x, y }).collect();
        self.place(&todo, &pts);
    }

    fn area(&self) -> (i32, i32, i32, i32) {
        let mut area = RECT::default();
        let _ =
            unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut area as *mut _ as _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)) };
        let (vx, vy) = unsafe { (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN)) };
        (area.left - vx, area.top - vy, area.right - area.left, area.bottom - area.top)
    }

    fn walls(screen: &[RECT]) -> Option<(POINT, Vec<Wall>)> {
        let mut o = POINT::default();
        let _ = unsafe { ClientToScreen(listview()?, &mut o) };
        Some((o, screen.iter().map(|r| (r.left - o.x, r.top - o.y, r.right - o.x, r.bottom - o.y)).collect()))
    }

    pub fn item_at(&self, (x, y): (i32, i32)) -> bool {
        let Some((o, _)) = Self::walls(&[]) else { return true };
        let (cell, p) = (self.spacing(), (x - o.x, y - o.y));
        self.entries()
            .iter()
            .filter(|e| !away(e.pos))
            .any(|e| covered(&[(e.pos.x, e.pos.y, e.pos.x + cell.x, e.pos.y + cell.y)], p, (1, 1)))
    }

    pub fn uncover(&self, paths: &[PathBuf], fences: &[RECT]) {
        if self.flags().is_some_and(|f| f & FWF_AUTOARRANGE.0 as u32 != 0) {
            return;
        }
        let Some((o, walls)) = Self::walls(fences) else { return };
        let (entries, cell) = (self.entries(), self.spacing());
        let size = (cell.x, cell.y);
        let mut taken: Vec<(i32, i32)> = entries.iter().filter(|e| !away(e.pos)).map(|e| (e.pos.x, e.pos.y)).collect();
        for e in entries.iter().filter(|e| listed(paths, &e.path) && !away(e.pos) && covered(&walls, (e.pos.x, e.pos.y), size)) {
            let (sx, sy) = (e.pos.x + o.x, e.pos.y + o.y);
            let a = work_area(&RECT { left: sx, top: sy, right: sx + 1, bottom: sy + 1 });
            let area = (a.left - o.x, a.top - o.y, a.right - a.left, a.bottom - a.top);
            if let Some(p) = clear_spot(&taken, &walls, size, area) {
                taken.push(p);
                self.place(&[e], &[POINT { x: p.0, y: p.1 }]);
            }
        }
    }

    pub fn sync(&self, keep: &[PathBuf], prev: Option<(i32, i32)>, fences: &[RECT]) -> Option<((i32, i32), RECT)> {
        let entries = self.entries();
        let cell = self.spacing();
        let icons: Vec<Icon> = entries.iter().map(|e| Icon { pos: (e.pos.x, e.pos.y), kept: listed(keep, &e.path) }).collect();
        let walls = Self::walls(fences).map(|(_, w)| w).unwrap_or_default();
        let plan = stack::plan(&icons, prev, (cell.x, cell.y), self.area(), &walls);
        let at = POINT { x: plan.stack.0, y: plan.stack.1 };
        if !plan.hide.is_empty() {
            if self.flags().is_some_and(|f| f & MASK != 0) {
                self.set_flags(0);
            }
            self.place(&plan.hide.iter().map(|&k| &entries[k]).collect::<Vec<_>>(), &vec![at; plan.hide.len()]);
        }
        let selected = |e: &Entry| unsafe { self.0.GetSelectionState(e.pidl) }.is_ok_and(|s| s & SVSI_SELECT.0 as u32 != 0);
        let picked: Vec<*const ITEMIDLIST> =
            plan.stacked.iter().map(|&k| &entries[k]).filter(|e| selected(e)).map(|e| e.pidl as *const _).collect();
        if !picked.is_empty() {
            let _ = unsafe { self.0.SelectAndPositionItems(picked.len() as u32, picked.as_ptr(), None, SVSI_DESELECT.0 as u32) };
        }
        if !plan.show.is_empty() {
            let items: Vec<&Entry> = plan.show.iter().map(|&(k, _)| &entries[k]).collect();
            self.place(&items, &plan.show.iter().map(|&(_, (x, y))| POINT { x, y }).collect::<Vec<_>>());
        }
        let lv = listview()?;
        let mut p = at;
        let _ = unsafe { ClientToScreen(lv, &mut p) };
        plan.any_kept.then_some((plan.stack, RECT { left: p.x - cell.x / 4, top: p.y, right: p.x + cell.x * 5 / 4, bottom: p.y + cell.y }))
    }
}
