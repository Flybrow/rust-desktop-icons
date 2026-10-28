use crate::report;
use windows::{
    Win32::{
        Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0},
        System::Threading::{CreateMutexW, WaitForSingleObject},
    },
    core::w,
};

const WAIT_MS: u32 = 15_000;

pub fn claim(relaunch: bool) -> Option<HANDLE> {
    let name = if cfg!(debug_assertions) { w!("Local\\RustDesktopIcons.dev") } else { w!("Local\\RustDesktopIcons") };
    let h = unsafe { CreateMutexW(None, true, name) }.ok()?;
    if unsafe { GetLastError() } != ERROR_ALREADY_EXISTS {
        return Some(h);
    }
    if !relaunch {
        return None;
    }
    let r = unsafe { WaitForSingleObject(h, WAIT_MS) };
    if r == WAIT_OBJECT_0 || r == WAIT_ABANDONED {
        return Some(h);
    }
    report::log("relaunch skipped: another instance is still running");
    None
}
