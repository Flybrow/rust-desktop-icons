use crate::{guard::alive, shared::key};
use std::{
    collections::HashSet,
    ffi::c_void,
    sync::{
        OnceLock, RwLock,
        atomic::{AtomicU32, Ordering::SeqCst},
    },
};
use windows::{
    Win32::{
        Foundation::{E_NOINTERFACE, E_NOTIMPL, E_POINTER, HWND, LPARAM, S_FALSE, S_OK, WPARAM},
        System::Com::CoTaskMemFree,
        UI::Shell::{Common::*, *},
    },
    core::*,
};

pub static LIST: RwLock<Option<HashSet<String>>> = RwLock::new(None);
pub static FILTERED: AtomicU32 = AtomicU32::new(0);

type Qi = unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT;
type Count = unsafe extern "system" fn(*mut c_void) -> u32;

#[repr(C)]
struct CbVtbl {
    qi: Qi,
    add: Count,
    release: Count,
    message: unsafe extern "system" fn(*mut c_void, u32, WPARAM, LPARAM) -> HRESULT,
}

#[repr(C)]
struct FfVtbl {
    qi: Qi,
    add: Count,
    release: Count,
    show: unsafe extern "system" fn(*mut c_void, *mut c_void, *const ITEMIDLIST, *const ITEMIDLIST) -> HRESULT,
    flags: unsafe extern "system" fn(*mut c_void, *mut c_void, *const ITEMIDLIST, *mut HWND, *mut u32) -> HRESULT,
}

#[repr(C)]
struct Wrap {
    cb: &'static CbVtbl,
    ff: &'static FfVtbl,
    refs: AtomicU32,
    old: OnceLock<IShellFolderViewCB>,
    filter: OnceLock<Option<IFolderFilter>>,
}

static CB: CbVtbl = CbVtbl { qi: cb_qi, add: cb_add, release: cb_release, message };
static FF: FfVtbl = FfVtbl { qi: ff_qi, add: ff_add, release: ff_release, show, flags };

unsafe fn base(this: *mut c_void, ff: bool) -> &'static Wrap {
    &*((this as *mut *const c_void).sub(ff as usize) as *const Wrap)
}

unsafe fn query(w: &Wrap, riid: *const GUID, ppv: *mut *mut c_void) -> HRESULT {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    let me = w as *const Wrap as *mut *const c_void;
    let out = match *riid {
        IUnknown::IID | IShellFolderViewCB::IID => me as *mut c_void,
        IFolderFilter::IID => me.add(1) as *mut c_void,
        _ => {
            *ppv = std::ptr::null_mut();
            return w.old.get().map_or(E_NOINTERFACE, |o| o.query(&*riid, ppv));
        }
    };
    w.refs.fetch_add(1, SeqCst);
    *ppv = out;
    S_OK
}

unsafe fn release(w: &Wrap) -> u32 {
    let n = w.refs.fetch_sub(1, SeqCst) - 1;
    if n == 0 {
        drop(Box::from_raw(w as *const Wrap as *mut Wrap));
    }
    n
}

unsafe extern "system" fn cb_qi(this: *mut c_void, riid: *const GUID, ppv: *mut *mut c_void) -> HRESULT {
    query(base(this, false), riid, ppv)
}
unsafe extern "system" fn ff_qi(this: *mut c_void, riid: *const GUID, ppv: *mut *mut c_void) -> HRESULT {
    query(base(this, true), riid, ppv)
}
unsafe extern "system" fn cb_add(this: *mut c_void) -> u32 {
    base(this, false).refs.fetch_add(1, SeqCst) + 1
}
unsafe extern "system" fn ff_add(this: *mut c_void) -> u32 {
    base(this, true).refs.fetch_add(1, SeqCst) + 1
}
unsafe extern "system" fn cb_release(this: *mut c_void) -> u32 {
    release(base(this, false))
}
unsafe extern "system" fn ff_release(this: *mut c_void) -> u32 {
    release(base(this, true))
}

unsafe extern "system" fn message(this: *mut c_void, msg: u32, w: WPARAM, l: LPARAM) -> HRESULT {
    match base(this, false).old.get() {
        Some(o) => (Interface::vtable(o).MessageSFVCB)(o.as_raw(), msg, w, l),
        None => E_NOTIMPL,
    }
}

fn hidden(psf: *mut c_void, item: *const ITEMIDLIST) -> bool {
    if !alive() {
        return false;
    }
    let Ok(list) = LIST.read() else { return false };
    let Some(list) = list.as_ref().filter(|l| !l.is_empty()) else { return false };
    unsafe {
        let Some(folder) = IShellFolder::from_raw_borrowed(&psf) else { return false };
        let mut sr = STRRET::default();
        if folder.GetDisplayNameOf(item, SHGDN_FORPARSING, &mut sr).is_err() {
            return false;
        }
        let mut s = PWSTR::null();
        if StrRetToStrW(&mut sr, Some(item), &mut s).is_err() {
            return false;
        }
        let text = s.to_string().ok();
        CoTaskMemFree(Some(s.0 as _));
        text.is_some_and(|t| list.contains(&key(&t)))
    }
}

unsafe extern "system" fn show(this: *mut c_void, psf: *mut c_void, folder: *const ITEMIDLIST, item: *const ITEMIDLIST) -> HRESULT {
    if let Some(Some(f)) = base(this, true).filter.get() {
        let hr = (Interface::vtable(f).ShouldShow)(f.as_raw(), psf, folder, item);
        if hr != S_OK {
            return hr;
        }
    }
    if hidden(psf, item) {
        FILTERED.fetch_add(1, SeqCst);
        return S_FALSE;
    }
    S_OK
}

unsafe extern "system" fn flags(this: *mut c_void, psf: *mut c_void, folder: *const ITEMIDLIST, hwnd: *mut HWND, out: *mut u32) -> HRESULT {
    match base(this, true).filter.get() {
        Some(Some(f)) => (Interface::vtable(f).GetEnumFlags)(f.as_raw(), psf, folder, hwnd, out),
        _ => E_NOTIMPL,
    }
}

pub fn install(view: &IShellFolderView) -> bool {
    let w = Box::into_raw(Box::new(Wrap { cb: &CB, ff: &FF, refs: AtomicU32::new(1), old: OnceLock::new(), filter: OnceLock::new() }));
    unsafe {
        let me = IShellFolderViewCB::from_raw(w as *mut c_void);
        let mut raw = std::ptr::null_mut();
        if (Interface::vtable(view).SetCallback)(view.as_raw(), me.as_raw(), &mut raw).is_err() {
            return false;
        }
        if raw.is_null() {
            return true;
        }
        let old = IShellFolderViewCB::from_raw(raw);
        let _ = (*w).filter.set(old.cast::<IFolderFilter>().ok());
        let _ = (*w).old.set(old);
    }
    true
}
