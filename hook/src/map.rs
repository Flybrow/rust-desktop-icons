use crate::shared::*;
use std::{
    collections::HashSet,
    sync::atomic::{AtomicU32, Ordering::SeqCst},
};
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
        System::Memory::*,
    },
    core::HSTRING,
};

pub struct Map {
    handle: HANDLE,
    view: MEMORY_MAPPED_VIEW_ADDRESS,
}

impl Drop for Map {
    fn drop(&mut self) {
        unsafe {
            let _ = UnmapViewOfFile(self.view);
            let _ = CloseHandle(self.handle);
        }
    }
}

impl Map {
    fn wrap(handle: HANDLE) -> Option<Self> {
        let view = unsafe { MapViewOfFile(handle, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, BYTES) };
        if view.Value.is_null() {
            let _ = unsafe { CloseHandle(handle) };
            return None;
        }
        Some(Self { handle, view })
    }

    pub fn create(pid: u32) -> Option<Self> {
        let h = unsafe { CreateFileMappingW(INVALID_HANDLE_VALUE, None, PAGE_READWRITE, 0, BYTES as u32, &HSTRING::from(name(pid))) };
        Self::wrap(h.ok()?)
    }

    pub fn open(pid: u32) -> Option<Self> {
        let h = unsafe { OpenFileMappingW((FILE_MAP_READ | FILE_MAP_WRITE).0, false, &HSTRING::from(name(pid))) };
        Self::wrap(h.ok()?)
    }

    fn head(&self) -> &[AtomicU32; HEAD] {
        unsafe { &*(self.view.Value as *const [AtomicU32; HEAD]) }
    }

    fn data(&self) -> *mut u16 {
        unsafe { (self.view.Value as *mut u8).add(HEAD * 4) as *mut u16 }
    }

    pub fn get(&self, i: usize) -> u32 {
        self.head()[i].load(SeqCst)
    }

    pub fn set(&self, i: usize, v: u32) {
        self.head()[i].store(v, SeqCst);
    }

    pub fn bump(&self, i: usize, n: u32) {
        self.head()[i].fetch_add(n, SeqCst);
    }

    pub fn write<S: AsRef<str>>(&self, paths: &[S]) {
        let data = encode(paths);
        let h = self.head();
        h[SEQ].fetch_add(1, SeqCst);
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), self.data(), data.len()) };
        h[LEN].store(data.len() as u32, SeqCst);
        h[SEQ].fetch_add(1, SeqCst);
    }

    pub fn read(&self) -> Option<HashSet<String>> {
        let h = self.head();
        let before = h[SEQ].load(SeqCst);
        if before % 2 == 1 {
            return None;
        }
        let len = (h[LEN].load(SeqCst) as usize).min(CHARS);
        let mut buf = vec![0u16; len];
        unsafe { std::ptr::copy_nonoverlapping(self.data(), buf.as_mut_ptr(), len) };
        (h[SEQ].load(SeqCst) == before).then(|| decode(&buf))
    }
}
