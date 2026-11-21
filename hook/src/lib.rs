pub mod exit;
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

fn registered(cell: &OnceLock<u32>, name: &str) -> u32 {
    *cell.get_or_init(|| unsafe { RegisterWindowMessageW(&HSTRING::from(name)) })
}

pub fn message() -> u32 {
    static M: OnceLock<u32> = OnceLock::new();
    registered(&M, MESSAGE)
}

pub fn reply() -> u32 {
    static M: OnceLock<u32> = OnceLock::new();
    registered(&M, REPLY)
}

pub fn post(view: HWND, map: &Map) -> Option<u32> {
    let before = map.get(PASSES);
    map.set(STAGE, 0);
    unsafe { PostMessageW(Some(view), message(), WPARAM(std::process::id() as usize), LPARAM(0)) }.ok()?;
    Some(before)
}

pub fn wait(map: &Map, before: u32, timeout: Duration) -> bool {
    let t = Instant::now();
    while map.get(PASSES) == before && t.elapsed() < timeout {
        thread::sleep(Duration::from_millis(2));
    }
    map.get(PASSES) != before && map.get(STAGE) == DONE
}

pub fn command(view: HWND, map: &Map, timeout: Duration) -> bool {
    post(view, map).is_some_and(|before| wait(map, before, timeout))
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
    let tray = map.get(TRAY);
    if tray != 0 {
        let _ = unsafe { PostMessageW(Some(HWND(tray as i32 as isize as _)), reply(), WPARAM(0), LPARAM(0)) };
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_needs_a_new_pass_that_finished() {
        let map = Map::create(u32::MAX - 1).unwrap();
        map.set(STAGE, DONE);
        assert!(!wait(&map, map.get(PASSES), Duration::from_millis(20)), "a stale DONE is not an answer");
        let before = map.get(PASSES);
        map.set(STAGE, 3);
        map.bump(PASSES, 1);
        assert!(!wait(&map, before, Duration::from_millis(20)), "a pass that stopped early failed");
        let before = map.get(PASSES);
        let t = thread::spawn(|| {
            let open = Map::open(u32::MAX - 1).unwrap();
            thread::sleep(Duration::from_millis(30));
            open.set(STAGE, DONE);
            open.bump(PASSES, 1);
        });
        assert!(wait(&map, before, Duration::from_secs(5)));
        t.join().unwrap();
        assert_ne!(message(), reply());
        assert!(message() >= 0xC000);
    }
}
