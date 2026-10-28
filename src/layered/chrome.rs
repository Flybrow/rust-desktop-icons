use super::{Frame, WHITE};
use crate::render::{flat, premul};
use windows::Win32::Foundation::RECT;

pub const BG: u32 = 0x24_2220;
pub const BG_END: u32 = 0x1A_1817;
pub const TEXT: u32 = 0xF2_F2F2;
pub const LINK: u32 = 0xFF_CD60;
pub const TOP: i32 = 56;
const CLOSE_HOT: u32 = 0x2B_11E8;

pub fn close_rect(w: i32, s: impl Fn(i32) -> i32) -> RECT {
    RECT { left: w - s(44), top: s(12), right: w - s(14), bottom: s(42) }
}

pub fn inside(r: &RECT, (x, y): (i32, i32)) -> bool {
    (r.left..r.right).contains(&x) && (r.top..r.bottom).contains(&y)
}

pub fn background(frame: &mut Frame, close_hot: bool, s: impl Fn(i32) -> i32) {
    let (w, ht, top, close) = (frame.w, frame.h, s(TOP), close_rect(frame.w, &s));
    let mut c = frame.canvas();
    c.rrect((0, 0, w, ht), s(12) as f32, (premul(BG, 252), premul(BG_END, 252)), (0, ht), false);
    c.rrect((0, top - 1, w, top), 0., flat(premul(WHITE, 26)), (0, ht), false);
    if close_hot {
        c.rrect((close.left, close.top, close.right, close.bottom), s(6) as f32, flat(premul(CLOSE_HOT, 230)), (0, top), false);
    }
}

pub fn outline(frame: &mut Frame, s: impl Fn(i32) -> i32) {
    let (w, ht) = (frame.w, frame.h);
    frame.canvas().rrect((0, 0, w, ht), s(12) as f32, flat(premul(WHITE, 40)), (0, ht), true);
}
