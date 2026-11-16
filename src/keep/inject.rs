use crate::{report, shell::hide::listview, store};
use rdi_hook::{command, map::Map, shared::*};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, fs, path::PathBuf, time::Duration};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::{GetProcAddress, LoadLibraryW},
        UI::WindowsAndMessaging::*,
    },
    core::{HSTRING, s},
};

type Proc = unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT;

static DLL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rdi_hook.dll"));

struct State {
    map: Map,
    hook: Option<(HHOOK, isize)>,
    sent: Option<Vec<String>>,
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

fn hook(view: HWND) -> Option<HHOOK> {
    unsafe {
        let m = LoadLibraryW(&HSTRING::from(dll()?.as_os_str())).ok()?;
        let call = GetProcAddress(m, s!("rdi_get_msg"))?;
        let tid = GetWindowThreadProcessId(view, None);
        let proc: HOOKPROC = Some(std::mem::transmute::<unsafe extern "system" fn() -> isize, Proc>(call));
        SetWindowsHookExW(WH_GETMESSAGE, proc, Some(m.into()), tid).ok()
    }
}

fn unhook(s: &mut State) {
    if let Some((h, _)) = s.hook.take() {
        let _ = unsafe { UnhookWindowsHookEx(h) };
    }
}

fn send(map: &Map, view: HWND) -> bool {
    command(view, map, Duration::from_secs(2))
}

fn session(view: HWND) -> Option<State> {
    let mut s = match STATE.take() {
        Some(s) => s,
        None => State { map: Map::create(std::process::id())?, hook: None, sent: None },
    };
    if s.hook.is_some_and(|(_, v)| v != view.0 as isize) {
        unhook(&mut s);
        s.sent = None;
    }
    if s.hook.is_none() {
        match hook(view) {
            Some(h) => s.hook = Some((h, view.0 as isize)),
            None => report::log("cannot load the desktop hook"),
        }
    }
    Some(s)
}

pub fn push(paths: &[PathBuf]) -> bool {
    let Some(view) = defview() else { return false };
    let Some(mut s) = session(view) else { return false };
    let keys: Vec<String> = paths.iter().map(|p| key(&p.to_string_lossy())).collect();
    let ok = s.hook.is_some()
        && (s.sent.as_ref() == Some(&keys) || {
            s.map.write(&keys);
            let ok = send(&s.map, view);
            if !ok {
                report::log(&format!("desktop hook failed at stage {} ({} errors)", s.map.get(STAGE), s.map.get(ERRORS)));
            }
            s.sent = ok.then_some(keys);
            ok
        });
    STATE.set(Some(s));
    ok
}

pub fn stop() {
    let Some(mut s) = STATE.take() else { return };
    if let Some((_, v)) = s.hook {
        s.map.write::<&str>(&[]);
        send(&s.map, HWND(v as _));
    }
    unhook(&mut s);
}
