mod filter;
mod guard;
pub mod map;
pub mod shared;
mod view;

use map::Map;
use shared::*;
use std::{
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU32, Ordering::SeqCst},
    },
    thread,
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::WindowsAndMessaging::*,
    },
    core::HSTRING,
};

static PID: AtomicU32 = AtomicU32::new(0);
static HIDDEN: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn message() -> u32 {
    static M: OnceLock<u32> = OnceLock::new();
    *M.get_or_init(|| unsafe { RegisterWindowMessageW(&HSTRING::from(MESSAGE)) })
}

pub fn command(view: HWND, map: &Map, wait: Duration) -> bool {
    let before = map.get(PASSES);
    map.set(STAGE, 0);
    if unsafe { PostMessageW(Some(view), message(), WPARAM(std::process::id() as usize), LPARAM(0)) }.is_err() {
        return false;
    }
    let t = Instant::now();
    while map.get(PASSES) == before && t.elapsed() < wait {
        thread::sleep(Duration::from_millis(2));
    }
    map.get(STAGE) == DONE
}

fn run(defview: HWND) {
    let pid = PID.load(SeqCst);
    let Some(map) = Map::open(pid) else { return };
    let t = Instant::now();
    if !guard::pin() {
        map.set(STAGE, NOT_PINNED);
    } else {
        guard::watch(pid);
        let ok = HIDDEN.try_lock().ok().and_then(|mut hidden| view::pass(defview, &map, &mut hidden));
        if ok.is_none() {
            map.bump(ERRORS, 1);
        }
    }
    map.set(MICROS, t.elapsed().as_micros() as u32);
    map.bump(PASSES, 1);
}

/// # Safety
/// WH_GETMESSAGE hook procedure, called by Windows only.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn rdi_get_msg(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32
        && w.0 == PM_REMOVE.0 as usize
        && let Some(m) = (l.0 as *const MSG).as_ref().filter(|m| m.message == message() && m.wParam.0 != 0)
    {
        if PID.swap(m.wParam.0 as u32, SeqCst) != m.wParam.0 as u32
            && let Ok(mut hidden) = HIDDEN.try_lock()
        {
            hidden.clear();
        }
        run(m.hwnd);
    }
    CallNextHookEx(None, code, w, l)
}
