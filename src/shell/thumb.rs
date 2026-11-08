use crate::win::wide;
use std::path::Path;
use windows::{
    Win32::{
        Foundation::SIZE,
        Graphics::Gdi::{
            BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC,
        },
        UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_THUMBNAILONLY},
    },
    core::PCWSTR,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Thumb {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u32>,
}

pub fn thumbnail(p: &Path, size: i32) -> Option<Thumb> {
    let path = wide(&p.to_string_lossy());
    unsafe {
        let f: IShellItemImageFactory = SHCreateItemFromParsingName(PCWSTR(path.as_ptr()), None).ok()?;
        let bmp = f.GetImage(SIZE { cx: size, cy: size }, SIIGBF_THUMBNAILONLY | SIIGBF_BIGGERSIZEOK).ok()?;
        let mut info = BITMAP::default();
        let got = GetObjectW(bmp.into(), size_of::<BITMAP>() as i32, Some(&mut info as *mut _ as _));
        let (w, h) = (info.bmWidth, info.bmHeight.abs());
        let mut px = vec![0u32; (w.max(0) * h) as usize];
        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let dc = GetDC(None);
        let lines = if got > 0 && w > 0 && h > 0 {
            GetDIBits(dc, bmp, 0, h as u32, Some(px.as_mut_ptr().cast()), &mut bi, DIB_RGB_COLORS)
        } else {
            0
        };
        ReleaseDC(None, dc);
        let _ = DeleteObject(bmp.into());
        if px.iter().all(|p| p >> 24 == 0) {
            px.iter_mut().for_each(|p| *p |= 0xFF00_0000);
        }
        (lines == h).then_some(Thumb { w, h, px })
    }
}
