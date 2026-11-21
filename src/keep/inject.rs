use crate::{
    domain::flight::{Flight, Next, WAIT_MS},
    report,
    shell::hide::listview,
    store, tray,
    win::now_ms,
};
use rdi_hook::{
    command,
    exit::{Watch, watch},
    map::Map,
    post,
    shared::*,
    wait,
};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, ffi::c_void, fs, path::PathBuf, time::Duration};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::{GetProcAddress, LoadLibraryW},
        UI::WindowsAndMessaging::*,
    },
    core::{HSTRING, s},
};

pub use rdi_hook::reply;

type Proc = unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT;

static DLL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rdi_hook.dll"));
const WAIT: Duration = Duration::from_millis(WAIT_MS);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Push {
    Done,
    Pending,
    Absent,
    Failed,
}

struct Hook {
    handle: HHOOK,
    view: isize,
    _explorer: Option<Watch>,
}

impl Drop for Hook {
    fn drop(&mut self) {
        let _ = unsafe { UnhookWindowsHookEx(self.handle) };
    }
}

struct State {
    map: Map,
    hook: Option<Hook>,
    flight: Flight,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn dll() -> Option<PathBuf> {
    let hash: String = Sha256::digest(DLL).iter().take(6).map(|b| format!("{b:02x}")).collect();
    let dir = store::root().join("hook");
    let path = dir.join(format!("rdi_hook-{hash}.dll"));
    if !path.exists() {
        let tmp = dir.join(format!("rdi_hook-{hash}.tmp"));
        fs::create_dir_all(&dir).ok()?;
        fs::write(&tmp, DLL).ok()?;
        fs::rename(&tmp, &path).ok()?;
        for old in fs::read_dir(&dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p != &path) {
            let _ = fs::remove_file(old);
        }
    }
    Some(path)
}

fn defview() -> Option<HWND> {
    unsafe { GetParent(listview()?).ok() }
}

unsafe extern "system" fn gone(_: *mut c_void, _: bool) {
    super::keeper::wake();
}

fn hook(view: HWND) -> Option<Hook> {
    unsafe {
        let m = LoadLibraryW(&HSTRING::from(dll()?.as_os_str())).ok()?;
        let call = GetProcAddress(m, s!("rdi_get_msg"))?;
        let mut pid = 0;
        let tid = GetWindowThreadProcessId(view, Some(&mut pid));
        let proc: HOOKPROC = Some(std::mem::transmute::<unsafe extern "system" fn() -> isize, Proc>(call));
        let handle = SetWindowsHookExW(WH_GETMESSAGE, proc, Some(m.into()), tid).ok()?;
        Some(Hook { handle, view: view.0 as isize, _explorer: watch(pid, Some(gone)) })
    }
}

fn session(view: HWND) -> Option<State> {
    let mut s = match STATE.take() {
        Some(s) => s,
        None => State { map: Map::create(std::process::id())?, hook: None, flight: Flight::default() },
    };
    if s.hook.as_ref().is_some_and(|h| h.view != view.0 as isize) {
        s.hook = None;
        s.flight = Flight::default();
    }
    Some(s)
}

impl State {
    fn send(&mut self, view: HWND, keys: Vec<String>, now: u64) -> Push {
        if self.hook.is_none() {
            self.hook = hook(view);
        }
        if self.hook.is_none() {
            report::log("cannot load the desktop hook");
            self.flight.fail(now);
            return Push::Failed;
        }
        self.map.write(&keys);
        self.map.set(TRAY, tray::hwnd().map_or(0, |h| h.0 as isize as u32));
        match post(view, &self.map) {
            Some(before) => {
                self.flight.sending(keys, before, now);
                Push::Pending
            }
            None => {
                self.flight.fail(now);
                Push::Failed
            }
        }
    }

    fn push(&mut self, view: HWND, keys: Vec<String>) -> Push {
        let now = now_ms() as u64;
        match self.flight.next(&keys, self.map.get(PASSES), self.map.get(STAGE) == DONE, now) {
            Next::Wait => Push::Pending,
            Next::Done => Push::Done,
            Next::Failed => Push::Failed,
            Next::Broke => {
                report::log(&format!("desktop hook failed at stage {} ({} errors)", self.map.get(STAGE), self.map.get(ERRORS)));
                Push::Failed
            }
            Next::Send => self.send(view, keys, now),
        }
    }
}

pub fn push(paths: &[PathBuf]) -> Push {
    let Some(view) = defview() else { return Push::Absent };
    let Some(mut s) = session(view) else { return Push::Failed };
    let r = s.push(view, paths.iter().map(|p| key(&p.to_string_lossy())).collect());
    STATE.set(Some(s));
    r
}

pub fn stop() {
    let Some(s) = STATE.take() else { return };
    if let Some(h) = &s.hook {
        if let Some(before) = s.flight.pending() {
            wait(&s.map, before, WAIT);
        }
        s.map.write::<&str>(&[]);
        s.map.set(TRAY, 0);
        command(HWND(h.view as _), &s.map, WAIT);
    }
}
