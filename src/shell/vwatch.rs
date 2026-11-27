use super::vdesk::keys;
use crate::win::wide;
use std::{sync::Arc, thread};
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

struct Event(HANDLE);

unsafe impl Send for Event {}
unsafe impl Sync for Event {}

impl Drop for Event {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn event() -> Option<Event> {
    unsafe { CreateEventW(None, false, false, None) }.ok().map(Event)
}

pub struct Watcher(Arc<Event>);

impl Drop for Watcher {
    fn drop(&mut self) {
        let _ = unsafe { SetEvent(self.0.0) };
    }
}

fn open(path: &str) -> Option<HKEY> {
    let (p, mut k) = (wide(path), HKEY::default());
    unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(p.as_ptr()), None, KEY_NOTIFY, &mut k) }.is_ok().then_some(k)
}

fn close(keys: Vec<HKEY>) {
    for k in keys {
        let _ = unsafe { RegCloseKey(k) };
    }
}

pub fn watch(changed: impl Fn(bool) + Send + 'static) -> Option<Watcher> {
    let keys: Vec<HKEY> = keys().iter().filter_map(|k| open(k)).collect();
    let (stop, events) = (event().map(Arc::new), keys.iter().map_while(|_| event()).collect::<Vec<Event>>());
    let Some(stop) = stop.filter(|_| !keys.is_empty() && events.len() == keys.len()) else {
        close(keys);
        return None;
    };
    let raw_keys: Vec<isize> = keys.iter().map(|k| k.0 as isize).collect();
    let mine = stop.clone();
    thread::spawn(move || {
        let keys: Vec<HKEY> = raw_keys.into_iter().map(|k| HKEY(k as _)).collect();
        let all: Vec<HANDLE> = events.iter().map(|e| e.0).chain([mine.0]).collect();
        let alive = loop {
            let armed = keys
                .iter()
                .zip(&events)
                .all(|(k, e)| unsafe { RegNotifyChangeKeyValue(*k, false, REG_NOTIFY_CHANGE_LAST_SET, Some(e.0), true) }.is_ok());
            if !armed {
                break false;
            }
            match unsafe { WaitForMultipleObjects(&all, false, INFINITE) }.0.wrapping_sub(WAIT_OBJECT_0.0) as usize {
                i if i < events.len() => changed(true),
                i if i == events.len() => break true,
                _ => break false,
            }
        };
        close(keys);
        if !alive {
            changed(false);
        }
    });
    Some(Watcher(stop))
}
