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
use std::{cell::RefCell, ffi::c_void, path::PathBuf, time::Duration};
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Push {
    Done(usize),
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
    fit: usize,
    stuck: bool,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn defview() -> Option<HWND> {
    unsafe { GetParent(listview()?).ok() }
}

unsafe extern "system" fn gone(_: *mut c_void, _: bool) {
    super::keeper::wake();
}

fn explorer(view: HWND) -> Option<(u32, u32)> {
    let (mut pid, mut shell) = (0, 0);
    let tid = unsafe { GetWindowThreadProcessId(view, Some(&mut pid)) };
    unsafe { GetWindowThreadProcessId(GetShellWindow(), Some(&mut shell)) };
    (tid != 0 && pid != 0 && pid == shell).then_some((tid, pid))
}

fn hook(view: HWND) -> Option<Hook> {
    let (tid, pid) = explorer(view)?;
    unsafe {
        let m = LoadLibraryW(&HSTRING::from(super::dll::ensure(&store::root().join("hook"), DLL)?.as_os_str())).ok()?;
        let call = GetProcAddress(m, s!("rdi_get_msg"))?;
        let proc: HOOKPROC = Some(std::mem::transmute::<unsafe extern "system" fn() -> isize, Proc>(call));
        let handle = SetWindowsHookExW(WH_GETMESSAGE, proc, Some(m.into()), tid).ok()?;
        Some(Hook { handle, view: view.0 as isize, _explorer: watch(pid, Some(gone)) })
    }
}

fn session(view: HWND) -> Option<State> {
    let mut s = match STATE.take() {
        Some(s) => s,
        None => State { map: Map::create(std::process::id())?, hook: None, flight: Flight::default(), fit: 0, stuck: false },
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
        self.stuck = false;
        self.fit = self.map.write(&keys);
        if self.fit < keys.len() {
            report::log(&format!("desktop hook: {} pinned items do not fit in its list, they stay stacked", keys.len() - self.fit));
        }
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
            Next::Done => Push::Done(self.fit),
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

pub fn stop() -> bool {
    let Some(mut s) = STATE.take() else { return true };
    let Some(view) = s.hook.as_ref().map(|h| HWND(h.view as _)).filter(|v| unsafe { IsWindow(Some(*v)) }.as_bool()) else { return true };
    let most = Duration::from_millis(WAIT_MS);
    if let Some((before, ms)) = s.flight.pending() {
        wait(&s.map, before, Duration::from_millis(ms).min(most));
    }
    s.map.write::<&str>(&[]);
    s.map.set(TRAY, 0);
    if command(view, &s.map, most) {
        return true;
    }
    if !s.stuck {
        report::log(&format!("desktop hook did not show the pinned items again (stage {}), retrying", s.map.get(STAGE)));
    }
    s.stuck = true;
    s.flight = Flight::default();
    STATE.set(Some(s));
    false
}
