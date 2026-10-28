use crate::{
    app::with,
    dialog::{self, Button, Spec},
    i18n::T,
    report,
    update::{self, Release, permille},
};
use std::{
    cell::RefCell,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    thread,
    time::Duration,
};
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

pub const WM_UPDATE: u32 = WM_APP + 2;
static AUTO: AtomicBool = AtomicBool::new(false);
static BUSY: AtomicBool = AtomicBool::new(false);
static QUEUED: AtomicBool = AtomicBool::new(false);
static LATEST: AtomicU32 = AtomicU32::new(0);
static DISMISSED: Mutex<Option<[u64; 3]>> = Mutex::new(None);

thread_local! {
    static PENDING: RefCell<Option<Release>> = const { RefCell::new(None) };
}

enum Msg {
    Current,
    Failed,
    Available(Release),
    Progress,
    Installing,
    InstallFailed(u32),
}

fn post(h: isize, m: Msg) {
    let raw = Box::into_raw(Box::new(m));
    if unsafe { PostMessageW(Some(HWND(h as _)), WM_UPDATE, WPARAM(0), LPARAM(raw as isize)) }.is_err() {
        drop(unsafe { Box::from_raw(raw) });
    }
}

pub fn enable(on: bool) {
    AUTO.store(on, Ordering::Relaxed);
}

fn check(h: isize, manual: bool) {
    if BUSY.swap(true, Ordering::AcqRel) {
        return;
    }
    let msg = match update::check() {
        Ok(Some(r)) if manual || *DISMISSED.lock().unwrap_or_else(|e| e.into_inner()) != Some(r.version) => Some(Msg::Available(r)),
        Ok(Some(_)) => None,
        Ok(None) => manual.then_some(Msg::Current),
        Err(e) => {
            report::log(&format!("update check: {e}"));
            manual.then_some(Msg::Failed)
        }
    };
    BUSY.store(false, Ordering::Release);
    if let Some(m) = msg {
        post(h, m);
    }
}

pub fn spawn(h: HWND) {
    let h = h.0 as isize;
    thread::spawn(move || {
        let fake = cfg!(debug_assertions) && std::env::var_os("RDI_FAKE_VERSION").is_some();
        thread::sleep(Duration::from_secs(if fake { 2 } else { 30 }));
        loop {
            if AUTO.load(Ordering::Relaxed) {
                check(h, false);
            }
            thread::sleep(Duration::from_secs(6 * 3600));
        }
    });
}

pub fn check_now(h: HWND) {
    let h = h.0 as isize;
    thread::spawn(move || check(h, true));
}

fn t(k: T) -> String {
    with(|a| a.t(k).to_string()).unwrap_or_default()
}

fn ok() -> Vec<Button> {
    vec![Button { label: t(T::GotIt), act: dialog::close }]
}

fn notice(body: String) {
    dialog::show(Spec { title: "Rust Desktop Icons".into(), body, buttons: ok(), progress: None, dismiss: dialog::close });
}

fn later() {
    let version = PENDING.with(|p| p.borrow_mut().take().map(|r| r.version));
    *DISMISSED.lock().unwrap_or_else(|e| e.into_inner()) = version;
    dialog::close();
}

fn install() {
    let (Some(rel), Some(tray)) = (PENDING.with(|p| p.borrow_mut().take()), super::hwnd()) else { return };
    dialog::progress(Some(0.), t(T::Downloading).replace("{}", "0"));
    let h = tray.0 as isize;
    thread::spawn(move || {
        let step = |done, total| {
            LATEST.store(permille(done, total), Ordering::Release);
            if !QUEUED.swap(true, Ordering::AcqRel) {
                post(h, Msg::Progress);
            }
        };
        let path = match update::download(&rel, step) {
            Ok(p) => p,
            Err(e) => {
                report::log(&format!("update download: {e}"));
                return post(h, Msg::Failed);
            }
        };
        post(h, Msg::Installing);
        let code = update::install(&path).unwrap_or_else(|e| {
            report::log(&format!("msiexec: {e}"));
            0xFFFF
        });
        let _ = std::fs::remove_file(&path);
        if !update::installed(code) {
            post(h, Msg::InstallFailed(code));
        }
    });
}

fn offer(rel: Release) {
    let body = t(T::UpdateVersion).replace("{new}", &rel.label()).replace("{cur}", &update::current());
    let body = format!("{body}\n\n{}", rel.notes);
    PENDING.with(|p| *p.borrow_mut() = Some(rel));
    let buttons = vec![Button { label: t(T::Install), act: install }, Button { label: t(T::Later), act: later }];
    dialog::show(Spec { title: t(T::UpdateAvailable), body, buttons, progress: None, dismiss: later });
}

pub fn received(lp: LPARAM) {
    match *unsafe { Box::from_raw(lp.0 as *mut Msg) } {
        Msg::Current => notice(t(T::UpToDate)),
        Msg::Failed => notice(t(T::UpdateFailed)),
        Msg::Available(r) => offer(r),
        Msg::Progress => {
            QUEUED.store(false, Ordering::Release);
            let p = LATEST.load(Ordering::Acquire);
            dialog::progress(Some(p as f32 / 1000.), t(T::Downloading).replace("{}", &(p / 10).to_string()));
        }
        Msg::Installing => dialog::progress(None, t(T::Installing)),
        Msg::InstallFailed(code) => notice(t(T::ErrInstall).replace("{}", &code.to_string())),
    }
}
