use super::hwnd;
use windows::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer};

pub const RULES: usize = 7;
pub const KEEP_TICK: usize = 8;
pub const KEEP_SOON: usize = 9;
pub const SCREENS: usize = 10;
pub const DBLCLICK: usize = 11;
pub const PEEK: usize = 12;

pub fn set(id: usize, ms: Option<u32>) {
    let Some(t) = hwnd() else { return };
    match ms {
        Some(ms) => unsafe {
            SetTimer(Some(t), id, ms, None);
        },
        None => {
            let _ = unsafe { KillTimer(Some(t), id) };
        }
    }
}
