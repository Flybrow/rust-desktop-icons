use super::vdesk::keys;
use crate::win::wide;
use std::thread;
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
        System::{
            Registry::{
                HKEY, HKEY_CURRENT_USER, KEY_NOTIFY, REG_NOTIFY_CHANGE_LAST_SET, RegCloseKey, RegNotifyChangeKeyValue, RegOpenKeyExW,
            },
            Threading::{CreateEventW, INFINITE, SetEvent, WaitForMultipleObjects},
        },
    },
    core::PCWSTR,
};

pub struct Watcher(HANDLE);

impl Drop for Watcher {
    fn drop(&mut self) {
        let _ = unsafe { SetEvent(self.0) };
    }
}

fn open(path: &str) -> Option<HKEY> {
    let (p, mut k) = (wide(path), HKEY::default());
    unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(p.as_ptr()), None, KEY_NOTIFY, &mut k) }.is_ok().then_some(k)
}

fn close(keys: Vec<HKEY>, handles: Vec<HANDLE>) {
    for k in keys {
        let _ = unsafe { RegCloseKey(k) };
    }
    for e in handles {
        let _ = unsafe { CloseHandle(e) };
    }
}

pub fn watch(changed: impl Fn() + Send + 'static) -> Option<Watcher> {
    let keys: Vec<HKEY> = keys().iter().filter_map(|k| open(k)).collect();
    let stop = unsafe { CreateEventW(None, false, false, None) }.ok()?;
    let events: Vec<HANDLE> = keys.iter().filter_map(|_| unsafe { CreateEventW(None, false, false, None) }.ok()).collect();
    if keys.is_empty() || events.len() != keys.len() {
        close(keys, events.into_iter().chain([stop]).collect());
        return None;
    }
    let (raw_keys, raw_events, raw_stop): (Vec<isize>, Vec<isize>, isize) =
        (keys.iter().map(|k| k.0 as isize).collect(), events.iter().map(|e| e.0 as isize).collect(), stop.0 as isize);
    thread::spawn(move || {
        let keys: Vec<HKEY> = raw_keys.into_iter().map(|k| HKEY(k as _)).collect();
        let events: Vec<HANDLE> = raw_events.into_iter().map(|e| HANDLE(e as _)).collect();
        let all: Vec<HANDLE> = events.iter().copied().chain([HANDLE(raw_stop as _)]).collect();
        loop {
            let armed = keys
                .iter()
                .zip(&events)
                .all(|(k, e)| unsafe { RegNotifyChangeKeyValue(*k, false, REG_NOTIFY_CHANGE_LAST_SET, Some(*e), true) }.is_ok());
            let hit = unsafe { WaitForMultipleObjects(&all, false, INFINITE) };
            if !armed || hit.0.wrapping_sub(WAIT_OBJECT_0.0) as usize >= events.len() {
                break;
            }
            changed();
        }
        close(keys, all);
    });
    Some(Watcher(stop))
}
