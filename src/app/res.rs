use super::App;
use crate::{i18n::T, layered, win::*};
use windows::{
    Win32::{
        Graphics::Gdi::*,
        UI::WindowsAndMessaging::{GetMenuItemCount, HMENU},
    },
    core::*,
};

impl Drop for App {
    fn drop(&mut self) {
        for &(_, f) in &self.fonts {
            let _ = unsafe { DeleteObject(f.into()) };
        }
        for &(_, b) in &self.glyphs {
            let _ = unsafe { DeleteObject(b.into()) };
        }
    }
}

impl App {
    pub fn font(&mut self, px: i32, weight: u32) -> HFONT {
        self.cached(px * 1000 + weight as i32, w!("Segoe UI"), px, weight, CLEARTYPE_QUALITY)
    }

    pub fn fluent_label(&mut self, px: i32) -> HFONT {
        let f = self.cached(-(px * 1000 + 400), w!("Segoe UI Variable Text"), px, 400, ANTIALIASED_QUALITY);
        if self.variable(f) { f } else { self.cached(-(px * 1000 + 401), w!("Segoe UI"), px, 400, ANTIALIASED_QUALITY) }
    }

    fn variable(&self, f: HFONT) -> bool {
        static VARIABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        let probe = |f: HFONT| unsafe {
            let dc = CreateCompatibleDC(None);
            let old = SelectObject(dc, f.into());
            let mut name = [0u16; 64];
            let n = GetTextFaceW(dc, Some(&mut name)).max(0) as usize;
            SelectObject(dc, old);
            let _ = DeleteDC(dc);
            String::from_utf16_lossy(&name[..n.saturating_sub(1).min(64)]).starts_with("Segoe UI Variable")
        };
        *VARIABLE.get_or_init(|| probe(f))
    }

    pub fn fluent_font(&mut self, px: i32) -> HFONT {
        let f = self.cached(-(px * 1000 + 600), w!("Segoe UI Variable Display"), px, 600, ANTIALIASED_QUALITY);
        if self.variable(f) { f } else { self.cached(-(px * 1000 + 402), w!("Segoe UI"), px, 400, ANTIALIASED_QUALITY) }
    }

    pub fn icon_font(&mut self, px: i32) -> HFONT {
        self.cached(-px, w!("Segoe MDL2 Assets"), px, 400, CLEARTYPE_QUALITY)
    }

    pub fn glyph(&mut self, ch: char) -> HBITMAP {
        if let Some(&(_, b)) = self.glyphs.iter().find(|(c, _)| *c == ch) {
            return b;
        }
        let px = sys_scale(16);
        let b = layered::glyph::bitmap(self.icon_font(px * 3 / 4), ch, px, 0x40_4040).unwrap_or_default();
        self.glyphs.push((ch, b));
        b
    }

    pub fn entry(&mut self, m: HMENU, id: usize, k: T, checked: bool, ch: char) {
        let b = self.glyph(ch);
        item(m, id, self.t(k), checked);
        icon(m, id as u32, false, b);
    }

    pub fn sub(&mut self, m: HMENU, k: T, child: HMENU, ch: char) {
        let b = self.glyph(ch);
        submenu(m, self.t(k), child);
        icon(m, unsafe { GetMenuItemCount(Some(m)) }.max(1) as u32 - 1, true, b);
    }

    fn cached(&mut self, key: i32, face: PCWSTR, px: i32, weight: u32, quality: FONT_QUALITY) -> HFONT {
        if let Some(&(_, f)) = self.fonts.iter().find(|(k, _)| *k == key) {
            return f;
        }
        let f = unsafe {
            CreateFontW(
                -px,
                0,
                0,
                0,
                weight as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                quality,
                FF_DONTCARE.0 as u32,
                face,
            )
        };
        self.fonts.push((key, f));
        f
    }
}
