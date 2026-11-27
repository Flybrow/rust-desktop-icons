use crate::{
    filter::{self, FILTERED as SHOWN_NOT, LIST},
    guard::DEFVIEW,
    map::Map,
    shared::*,
};
use std::{
    collections::HashSet,
    path::Path,
    sync::{Arc, atomic::Ordering::SeqCst},
};
use windows::{
    Win32::{
        Foundation::HWND,
        System::{
            Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree, IServiceProvider},
            Variant::VARIANT,
        },
        UI::{
            Shell::{Common::*, *},
            WindowsAndMessaging::{FindWindowW, GetParent, SendMessageW, WM_USER},
        },
    },
    core::*,
};

const WM_GETISHELLBROWSER: u32 = WM_USER + 7;

struct Pidl(*mut ITEMIDLIST);

impl Drop for Pidl {
    fn drop(&mut self) {
        unsafe { CoTaskMemFree(Some(self.0 as _)) };
    }
}

fn note(map: Option<&Map>, stage: u32, hr: i32) {
    if let Some(m) = map {
        m.set(STAGE, stage);
        m.set(HR, hr as u32);
    }
}

fn record<T>(map: Option<&Map>, stage: u32, r: Result<T>) -> Option<T> {
    r.map_err(|e| note(map, stage, e.code().0)).ok()
}

fn from_browser(map: Option<&Map>, defview: HWND) -> Option<IShellView> {
    unsafe {
        let host = GetParent(defview).ok().unwrap_or_default();
        let progman = FindWindowW(w!("Progman"), None).unwrap_or_default();
        for h in [host, progman] {
            let raw = SendMessageW(h, WM_GETISHELLBROWSER, None, None).0 as *mut core::ffi::c_void;
            if let Some(b) = IShellBrowser::from_raw_borrowed(&raw) {
                return record(map, 12, b.QueryActiveShellView());
            }
        }
        note(map, 11, 0);
        None
    }
}

fn from_windows(map: Option<&Map>) -> Option<IShellView> {
    unsafe {
        let windows: IShellWindows = record(map, 21, CoCreateInstance(&ShellWindows, None, CLSCTX_ALL))?;
        let mut hwnd = 0;
        let loc = VARIANT::from(CSIDL_DESKTOP as i32);
        let disp = record(map, 22, windows.FindWindowSW(&loc, &VARIANT::default(), SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH))?;
        let sp = record(map, 23, disp.cast::<IServiceProvider>())?;
        let browser: IShellBrowser = record(map, 24, sp.QueryService(&SID_STopLevelBrowser))?;
        record(map, 25, browser.QueryActiveShellView())
    }
}

fn shell_view(map: Option<&Map>, defview: HWND) -> Option<IShellView> {
    from_browser(map, defview).or_else(|| from_windows(map))
}

pub fn display(folder: &IShellFolder, pidl: *const ITEMIDLIST) -> Option<String> {
    unsafe {
        let mut sr = STRRET::default();
        folder.GetDisplayNameOf(pidl, SHGDN_FORPARSING, &mut sr).ok()?;
        let mut s = PWSTR::null();
        StrRetToStrW(&mut sr, Some(pidl), &mut s).ok()?;
        let text = s.to_string().ok();
        CoTaskMemFree(Some(s.0 as _));
        text
    }
}

fn path(folder: &IShellFolder, pidl: *const ITEMIDLIST) -> Option<String> {
    display(folder, pidl).map(|t| key(&t))
}

fn items(view: &IFolderView) -> Vec<Pidl> {
    let mut out = vec![];
    let Ok(list) = (unsafe { view.Items::<IEnumIDList>(SVGIO_ALLVIEW) }) else { return out };
    loop {
        let mut pidl = [std::ptr::null_mut()];
        if unsafe { list.Next(&mut pidl, None) }.0 != 0 || pidl[0].is_null() {
            return out;
        }
        out.push(Pidl(pidl[0]));
    }
}

fn parse(folder: &IShellFolder, name: &str) -> Option<Pidl> {
    let mut pidl = std::ptr::null_mut();
    unsafe { folder.ParseDisplayName(HWND::default(), None, &HSTRING::from(name), None, &mut pidl, std::ptr::null_mut()) }.ok()?;
    Some(Pidl(pidl)).filter(|p| !p.0.is_null() && unsafe { ILFindLastID(p.0) == p.0 })
}

fn child(folder: &IShellFolder, full: &str) -> Option<Pidl> {
    let name = Path::new(full).file_name()?.to_str()?;
    [full, name].into_iter().filter_map(|n| parse(folder, n)).find(|p| path(folder, p.0).as_deref() == Some(full))
}

fn remove(sfv: &IShellFolderView, pidl: &Pidl, map: &Map) -> bool {
    let ok = unsafe { sfv.RemoveObject(Some(pidl.0)) }.is_ok();
    if ok {
        map.bump(REMOVED, 1);
    }
    ok
}

fn sweep(fv: &IFolderView, sfv: &IShellFolderView, folder: &IShellFolder, todo: &HashSet<&String>, map: &Map) -> bool {
    let all = items(fv);
    map.set(ITEMS, all.len() as u32);
    let found = all.iter().filter(|p| path(folder, p.0).is_some_and(|p| todo.contains(&p)));
    found.filter(|pidl| !remove(sfv, pidl, map)).count() == 0
}

pub fn pass(defview: HWND, map: &Map, hidden: &mut Vec<String>) -> Option<()> {
    let list = Arc::new(map.read()?);
    if let Ok(mut l) = LIST.write() {
        *l = Some(list.clone());
    }
    let view = shell_view(Some(map), defview)?;
    let fv = record(Some(map), 31, view.cast::<IFolderView>())?;
    let sfv = record(Some(map), 32, view.cast::<IShellFolderView>())?;
    if DEFVIEW.load(SeqCst) != defview.0 as isize && filter::install(&sfv) {
        DEFVIEW.store(defview.0 as isize, SeqCst);
        map.bump(INSTALLS, 1);
    }
    let folder: IShellFolder = record(Some(map), 33, unsafe { fv.GetFolder() })?;
    let mut refresh = false;
    let (show, hide) = changes(hidden, &list);
    for p in show.into_iter().filter(|p| Path::new(p).exists()) {
        match child(&folder, p) {
            Some(pidl) if unsafe { sfv.AddObject(pidl.0) }.is_ok() => map.bump(ADDED, 1),
            _ => refresh = true,
        }
    }
    let mut lost = HashSet::new();
    for p in hide.into_iter().filter(|p| Path::new(p).exists()) {
        match child(&folder, p) {
            Some(pidl) => refresh |= !remove(&sfv, &pidl, map),
            None => {
                lost.insert(p);
            }
        }
    }
    if !lost.is_empty() {
        map.bump(SWEEPS, 1);
        refresh |= !sweep(&fv, &sfv, &folder, &lost, map);
    }
    *hidden = list.iter().cloned().collect();
    map.set(HIDDEN, hidden.len() as u32);
    map.set(FILTERED, SHOWN_NOT.load(SeqCst));
    if refresh {
        map.bump(REFRESHES, 1);
        let _ = unsafe { view.Refresh() };
    }
    if DEFVIEW.load(SeqCst) == defview.0 as isize {
        map.set(STAGE, DONE);
    }
    Some(())
}

pub fn refresh(defview: HWND) {
    if let Some(v) = shell_view(None, defview) {
        let _ = unsafe { v.Refresh() };
    }
}
