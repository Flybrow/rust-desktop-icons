pub mod chrome;
mod dib;
pub mod glyph;
pub mod popup;

use crate::render::{Canvas, blur, gray, premul};
pub use dib::Dib;
use windows::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};

pub const WHITE: u32 = 0xFF_FFFF;
pub const ACCENT: u32 = 0xD4_7800;

pub struct Labels {
    pub key: u64,
    mask: Vec<u8>,
    soft: Vec<u8>,
}

pub struct Frame {
    pub w: i32,
    pub h: i32,
    pub canvas: Dib,
    pub mask: Dib,
    dc: HDC,
    old: HGDIOBJ,
}

pub fn measure(font: HFONT, text: &[u16], width: i32, flags: DRAW_TEXT_FORMAT) -> i32 {
    if text.is_empty() {
        return 0;
    }
    let (mut w, mut r) = (text.to_vec(), RECT { left: 0, top: 0, right: width, bottom: 0 });
    let dc = MEASURE.with(|c| {
        if c.get() == 0 {
            c.set(unsafe { CreateCompatibleDC(None) }.0 as isize);
        }
        HDC(c.get() as _)
    });
    unsafe {
        let old = SelectObject(dc, font.into());
        DrawTextW(dc, &mut w, &mut r, flags | DT_CALCRECT);
        SelectObject(dc, old);
    }
    r.bottom
}

thread_local! {
    static MEASURE: std::cell::Cell<isize> = const { std::cell::Cell::new(0) };
}

impl Frame {
    pub fn new(w: i32, h: i32) -> Option<Self> {
        let (canvas, mask) = (Dib::new(w, h)?, Dib::new(w, h)?);
        unsafe {
            let dc = CreateCompatibleDC(None);
            let old = SelectObject(dc, mask.bmp.into());
            SetBkMode(dc, TRANSPARENT);
            SetTextColor(dc, COLORREF(WHITE));
            Some(Self { w, h, canvas, mask, dc, old })
        }
    }

    pub fn dc(&self) -> HDC {
        self.dc
    }

    pub fn text(&self, font: HFONT, text: &str, mut r: RECT, flags: DRAW_TEXT_FORMAT) {
        let mut t: Vec<u16> = text.encode_utf16().collect();
        self.text_w(font, &mut t, &mut r, flags);
    }

    pub fn text_w(&self, font: HFONT, text: &mut [u16], r: &mut RECT, flags: DRAW_TEXT_FORMAT) {
        if text.is_empty() {
            return;
        }
        unsafe {
            SelectObject(self.dc, font.into());
            DrawTextW(self.dc, text, r, flags | DT_NOPREFIX);
        }
    }

    pub fn clip(&self, (x0, y0, x1, y1): (i32, i32, i32, i32)) {
        unsafe { IntersectClipRect(self.dc, x0, y0, x1, y1) };
    }

    pub fn unclip(&self) {
        unsafe { SelectClipRgn(self.dc, None) };
    }

    pub fn target(&self, dib: &Dib) {
        unsafe { SelectObject(self.dc, dib.bmp.into()) };
    }

    pub fn flush(&mut self) {
        self.sync();
        gray(self.mask.px());
    }

    pub fn sync(&self) {
        let _ = unsafe { GdiFlush() };
    }

    pub fn export_bitmap(&mut self) -> Option<HBITMAP> {
        let mut out = std::mem::ManuallyDrop::new(Dib::new(self.w, self.h)?);
        out.px().copy_from_slice(self.canvas.px());
        Some(out.bmp)
    }

    pub fn compact(&mut self) {
        unsafe { SelectObject(self.dc, self.canvas.bmp.into()) };
        if let Some(m) = (self.mask.px().len() > 1).then(|| Dib::new(1, 1)).flatten() {
            self.mask = m;
        }
    }

    pub fn clear_mask(&mut self) {
        self.mask.px().fill(0);
    }

    pub fn canvas(&mut self) -> Canvas<'_> {
        Canvas { px: self.canvas.px(), w: self.w, h: self.h }
    }

    pub fn labels(&mut self, key: u64, r: i32) -> Labels {
        let (w, h) = (self.w, self.h);
        let mask = self.mask.px();
        Labels { key, soft: blur(mask, w, h, r), mask: mask.iter().map(|p| *p as u8).collect() }
    }

    pub fn layers(&mut self, l: &Labels, fg: u32, shadow: u32, rows: (i32, i32)) {
        let mut c = self.canvas();
        c.layer(&l.soft, (0, 1), Some(premul(shadow, 190)), rows);
        c.layer(&l.mask, (0, 0), Some(premul(fg, 255)), rows);
    }

    pub fn text_layers(&mut self, fg: u32, shadow: u32, r: i32, rows: (i32, i32)) {
        let l = self.labels(0, r);
        self.layers(&l, fg, shadow, rows);
    }

    #[cfg(debug_assertions)]
    pub fn dump(&mut self, name: &str) {
        if let Some(p) = std::env::var_os("RDI_DUMP") {
            let head = [self.w.to_le_bytes(), self.h.to_le_bytes()].concat();
            let body = self.canvas.px().iter().flat_map(|v| v.to_le_bytes()).collect();
            let _ = std::fs::write(std::path::PathBuf::from(p).join(format!("{name}.bin")), [head, body].concat());
        }
    }

    pub fn present(&self, h: HWND, pos: (i32, i32), alpha: u8) {
        self.present_part(h, pos, (0, 0), (self.w, self.h), alpha);
    }

    pub fn present_part(&self, h: HWND, (x, y): (i32, i32), (sx, sy): (i32, i32), (w, ht): (i32, i32), alpha: u8) {
        let blend =
            BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: alpha, AlphaFormat: AC_SRC_ALPHA as u8 };
        let (pos, size) = (POINT { x, y }, SIZE { cx: w.clamp(1, self.w), cy: ht.clamp(1, self.h) });
        let src = POINT { x: sx.clamp(0, self.w - size.cx), y: sy.clamp(0, self.h - size.cy) };
        unsafe {
            SelectObject(self.dc, self.canvas.bmp.into());
            let _ = UpdateLayeredWindow(h, None, Some(&pos), Some(&size), Some(self.dc), Some(&src), COLORREF(0), Some(&blend), ULW_ALPHA);
        }
        #[cfg(debug_assertions)]
        self.mark_drawn(h, src, size, alpha);
    }

    #[cfg(debug_assertions)]
    fn mark_drawn(&self, h: HWND, src: POINT, size: SIZE, alpha: u8) {
        let px = self.canvas.pixels();
        let rows = (src.y..src.y + size.cy).map(|y| (y * self.w + src.x) as usize);
        let visible =
            alpha > 0 && rows.into_iter().any(|r| px.get(r..r + size.cx as usize).is_some_and(|row| row.iter().any(|p| p >> 24 != 0)));
        let _ = unsafe { SetPropW(h, windows::core::w!("RDI_DRAWN"), Some(HANDLE(visible as usize as _))) };
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteDC(self.dc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_with_text() -> Frame {
        let mut f = Frame::new(120, 60).unwrap();
        f.canvas().px.fill(0xFF_20_30_40);
        let font = HFONT(unsafe { GetStockObject(DEFAULT_GUI_FONT) }.0);
        f.text(font, "Rapport annuel", RECT { left: 4, top: 10, right: 116, bottom: 50 }, DT_CENTER | DT_WORDBREAK);
        f.flush();
        f
    }

    #[test]
    fn cached_labels_paint_the_same_pixels() {
        let mut a = frame_with_text();
        a.text_layers(WHITE, 0, 2, (0, 60));
        let mut b = frame_with_text();
        let labels = b.labels(7, 2);
        let mut c = Frame::new(120, 60).unwrap();
        c.canvas().px.fill(0xFF_20_30_40);
        c.layers(&labels, WHITE, 0, (0, 60));
        assert!(a.canvas.px().iter().any(|p| *p != 0xFF_20_30_40));
        assert_eq!(a.canvas.px(), c.canvas.px());
        assert_eq!(labels.key, 7);
    }
}
