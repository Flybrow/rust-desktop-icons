use std::{
    any::Any,
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::PostMessageW,
};

type Letter = (usize, Box<dyn Any + Send>);

static NEXT: AtomicUsize = AtomicUsize::new(1);
static BOX: Mutex<Vec<Letter>> = Mutex::new(Vec::new());

fn mailbox() -> MutexGuard<'static, Vec<Letter>> {
    BOX.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn post<T: Any + Send>(h: HWND, msg: u32, v: T) -> bool {
    let key = NEXT.fetch_add(1, Ordering::Relaxed);
    mailbox().push((key, Box::new(v)));
    let sent = unsafe { PostMessageW(Some(h), msg, WPARAM(0), LPARAM(key as isize)) }.is_ok();
    if !sent {
        drop(take::<T>(LPARAM(key as isize)));
    }
    sent
}

pub fn take<T: Any>(lp: LPARAM) -> Option<T> {
    let mut m = mailbox();
    let i = m.iter().position(|(k, v)| *k == lp.0 as usize && v.is::<T>())?;
    let letter = m.swap_remove(i).1;
    drop(m);
    letter.downcast().ok().map(|b| *b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_are_taken_once_by_key_and_type() {
        let key = NEXT.fetch_add(1, Ordering::Relaxed);
        mailbox().push((key, Box::new(String::from("hi"))));
        let lp = LPARAM(key as isize);
        assert_eq!((take::<u32>(lp), take::<String>(LPARAM(0)), take::<String>(LPARAM(-1))), (None, None, None));
        assert_eq!(take::<String>(lp).as_deref(), Some("hi"));
        assert_eq!(take::<String>(lp), None);
    }
}
