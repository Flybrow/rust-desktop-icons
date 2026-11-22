use windows::Win32::{
    Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
    System::Threading::*,
};

pub struct Watch {
    pub process: HANDLE,
    pub wait: HANDLE,
}

impl Drop for Watch {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterWaitEx(self.wait, Some(INVALID_HANDLE_VALUE));
            let _ = CloseHandle(self.process);
        }
    }
}

pub fn watch(pid: u32, done: WAITORTIMERCALLBACK) -> Option<Watch> {
    unsafe {
        let process = OpenProcess(PROCESS_SYNCHRONIZE, false, pid).ok()?;
        let mut wait = HANDLE::default();
        if RegisterWaitForSingleObject(&mut wait, process, done, None, INFINITE, WT_EXECUTEONLYONCE).is_err() {
            let _ = CloseHandle(process);
            return None;
        }
        Some(Watch { process, wait })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::windows::process::CommandExt,
        process::Command,
        sync::atomic::{AtomicU32, Ordering::SeqCst},
        thread,
        time::Duration,
    };

    static ENDED: AtomicU32 = AtomicU32::new(0);
    static STAYED: AtomicU32 = AtomicU32::new(0);

    unsafe extern "system" fn ended(_: *mut core::ffi::c_void, _: bool) {
        ENDED.fetch_add(1, SeqCst);
    }

    unsafe extern "system" fn stayed(_: *mut core::ffi::c_void, _: bool) {
        STAYED.fetch_add(1, SeqCst);
    }

    fn settle(n: &AtomicU32, want: u32) -> u32 {
        for _ in 0..200 {
            if n.load(SeqCst) == want {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        n.load(SeqCst)
    }

    #[test]
    fn calls_back_once_when_the_process_ends() {
        let mut child = Command::new("cmd").args(["/c", "exit"]).creation_flags(0x0800_0000).spawn().unwrap();
        child.wait().unwrap();
        let w = watch(child.id(), Some(ended)).unwrap();
        assert_eq!(settle(&ENDED, 1), 1);
        drop(w);
        assert_eq!(ENDED.load(SeqCst), 1);
        assert!(watch(u32::MAX - 3, Some(ended)).is_none());
    }

    #[test]
    fn a_dropped_watch_never_calls_back() {
        let mut child = Command::new("cmd").args(["/c", "ping -n 30 127.0.0.1 >nul"]).creation_flags(0x0800_0000).spawn().unwrap();
        drop(watch(child.id(), Some(stayed)).unwrap());
        child.kill().unwrap();
        child.wait().unwrap();
        thread::sleep(Duration::from_millis(200));
        assert_eq!(STAYED.load(SeqCst), 0);
    }
}
