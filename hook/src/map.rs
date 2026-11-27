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

    pub fn write<S: AsRef<str>>(&self, paths: &[S]) -> usize {
        let (data, n) = encode(paths);
        let h = self.head();
        h[SEQ].fetch_add(1, SeqCst);
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), self.data(), data.len()) };
        h[LEN].store(data.len() as u32, SeqCst);
        h[SEQ].fetch_add(1, SeqCst);
        n
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_sides_see_the_same_list_and_counters() {
        let a = Map::create(u32::MAX - 2).unwrap();
        let b = Map::open(u32::MAX - 2).unwrap();
        assert_eq!(a.write(&["C:/D/One.txt", "c:/d/two.lnk"]), 2);
        let list = b.read().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.contains("c:/d/one.txt") && list.contains("c:/d/two.lnk"));
        assert_eq!(b.get(SEQ) % 2, 0);
        b.bump(PASSES, 2);
        b.set(STAGE, DONE);
        assert_eq!((a.get(PASSES), a.get(STAGE)), (2, DONE));
        a.write::<&str>(&[]);
        assert!(b.read().unwrap().is_empty());
    }

    #[test]
    fn a_list_being_written_is_never_read() {
        let a = Map::create(u32::MAX - 4).unwrap();
        a.write(&["x"]);
        a.bump(SEQ, 1);
        assert!(a.read().is_none());
        a.bump(SEQ, 1);
        assert_eq!(a.read().unwrap().len(), 1);
        assert!(Map::open(u32::MAX - 5).is_none());
    }
}
