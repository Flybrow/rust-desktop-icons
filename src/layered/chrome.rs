use super::{Frame, WHITE};
use crate::render::{flat, premul};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use windows::Win32::Foundation::RECT;

pub struct Palette {
    pub bg: u32,
    pub bg_end: u32,
    pub text: u32,
    pub link: u32,
    pub dim: u32,
    pub ink: u32,
    pub shade: u32,
    pub track: u32,
    pub on_accent: u32,
}

const DARK: Palette = Palette {
    bg: 0x24_2220,
    bg_end: 0x1A_1817,
    text: 0xF2_F2F2,
    link: 0xFF_CD60,
    dim: 0xB4_B4B4,
    ink: WHITE,
    shade: 0,
    track: 0x55_5555,
    on_accent: 0xF2_F2F2,
};

const LIGHT: Palette = Palette {
    bg: 0xFB_FBFB,
    bg_end: 0xF1_F1F1,
    text: 0x1B_1B1B,
    link: 0xC0_6700,
    dim: 0x5C_5C5C,
    ink: 0,
    shade: WHITE,
    track: 0x9A_9A9A,
    on_accent: WHITE,
};

static IS_LIGHT: AtomicBool = AtomicBool::new(false);

pub fn set_light(on: bool) {
    IS_LIGHT.store(on, Relaxed);
}

pub fn palette() -> &'static Palette {
    if IS_LIGHT.load(Relaxed) { &LIGHT } else { &DARK }
}

pub const TOP: i32 = 56;
const CLOSE_HOT: u32 = 0x2B_11E8;

pub fn close_rect(w: i32, s: impl Fn(i32) -> i32) -> RECT {
    RECT { left: w - s(44), top: s(12), right: w - s(14), bottom: s(42) }
}

pub fn inside(r: &RECT, (x, y): (i32, i32)) -> bool {
    (r.left..r.right).contains(&x) && (r.top..r.bottom).contains(&y)
}

pub fn background(frame: &mut Frame, close_hot: bool, s: impl Fn(i32) -> i32) {
    let (w, ht, top, close, p) = (frame.w, frame.h, s(TOP), close_rect(frame.w, &s), palette());
    let mut c = frame.canvas();
    c.rrect((0, 0, w, ht), s(12) as f32, (premul(p.bg, 252), premul(p.bg_end, 252)), (0, ht), false);
    c.rrect((0, top - 1, w, top), 0., flat(premul(p.ink, 26)), (0, ht), false);
    if close_hot {
        c.rrect((close.left, close.top, close.right, close.bottom), s(6) as f32, flat(premul(CLOSE_HOT, 230)), (0, top), false);
    }
}

pub fn outline(frame: &mut Frame, s: impl Fn(i32) -> i32) {
    let (w, ht) = (frame.w, frame.h);
    frame.canvas().rrect((0, 0, w, ht), s(12) as f32, flat(premul(palette().ink, 40)), (0, ht), true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dark_palette_is_the_old_look_and_light_inverts_ink() {
        set_light(false);
        assert_eq!((palette().bg, palette().text, palette().ink, palette().shade), (0x24_2220, 0xF2_F2F2, WHITE, 0));
        set_light(true);
        assert_eq!((palette().ink, palette().shade, palette().on_accent), (0, WHITE, WHITE));
        assert!(palette().text < 0x40_4040);
        set_light(false);
    }
}
