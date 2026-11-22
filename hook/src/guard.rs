use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering::SeqCst};
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, HMODULE, HWND, LPARAM, LRESULT, WAIT_TIMEOUT, WPARAM},
        System::{LibraryLoader::*, Threading::*},
        UI::WindowsAndMessaging::*,
    },
    core::*,
};

static PID: AtomicU32 = AtomicU32::new(0);
static PROC: AtomicIsize = AtomicIsize::new(0);
static WAIT: AtomicIsize = AtomicIsize::new(0);
static WINDOW: AtomicIsize = AtomicIsize::new(0);
pub static DEFVIEW: AtomicIsize = AtomicIsize::new(0);
const DIED: u32 = WM_APP + 1;

pub fn alive() -> bool {
    let h = PROC.load(SeqCst);
    h != 0 && unsafe { WaitForSingleObject(HANDLE(h as _), 0) } == WAIT_TIMEOUT
}

pub fn pin() -> bool {
    let mut m = HMODULE::default();
    let flags = GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_PIN;
    unsafe { GetModuleHandleExW(flags, PCWSTR(pin as *const () as *const u16), &mut m) }.is_ok()
}

unsafe extern "system" fn proc(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == DIED {
        crate::view::refresh(HWND(DEFVIEW.load(SeqCst) as _));
        return LRESULT(0);
    }
    DefWindowProcW(h, msg, w, l)
}

fn window() -> Option<HWND> {
    let h = WINDOW.load(SeqCst);
    if h != 0 {
        return Some(HWND(h as _));
    }
    unsafe {
        let mut m = HMODULE::default();
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(proc as *const () as *const u16),
            &mut m,
        )
        .ok()?;
        let class = w!("RustDesktopIcons.HookWindow");
        let wc = WNDCLASSW { lpfnWndProc: Some(proc), hInstance: m.into(), lpszClassName: class, ..Default::default() };
        RegisterClassW(&wc);
        let h =
            CreateWindowExW(WINDOW_EX_STYLE(0), class, None, WINDOW_STYLE(0), 0, 0, 0, 0, Some(HWND_MESSAGE), None, Some(m.into()), None)
                .ok()?;
        WINDOW.store(h.0 as isize, SeqCst);
        Some(h)
    }
}

unsafe extern "system" fn died(_: *mut core::ffi::c_void, _: bool) {
    let h = WINDOW.load(SeqCst);
    if h != 0 {
        let _ = PostMessageW(Some(HWND(h as _)), DIED, WPARAM(0), LPARAM(0));
    }
}

pub fn watch(pid: u32) {
    if PID.swap(pid, SeqCst) == pid || window().is_none() {
        return;
    }
    if let Some(w) = crate::exit::watch(pid, Some(died)) {
        let (process, wait) = (w.process.0 as isize, w.wait.0 as isize);
        std::mem::forget(w);
        let (old, waited) = (PROC.swap(process, SeqCst), WAIT.swap(wait, SeqCst));
        unsafe {
            if waited != 0 {
                let _ = UnregisterWait(HANDLE(waited as _));
            }
            if old != 0 {
                let _ = CloseHandle(HANDLE(old as _));
            }
        }
    }
}
