use crate::{render::premultiplied, win::inst};
use std::{cell::RefCell, rc::Rc};
use windows::{
    Win32::{
        Graphics::Gdi::{BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDIBits},
        UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO, IMAGE_ICON, LR_DEFAULTCOLOR, LoadImageW},
    },
    core::PCWSTR,
};

thread_local! {
    static CACHE: RefCell<Option<(i32, Rc<Vec<u32>>)>> = const { RefCell::new(None) };
}

fn load(px: i32) -> Option<Vec<u32>> {
    let mut out = vec![0u32; (px * px) as usize];
    unsafe {
        let icon = HICON(LoadImageW(Some(inst()), PCWSTR(1 as _), IMAGE_ICON, px, px, LR_DEFAULTCOLOR).ok()?.0);
        let mut info = ICONINFO::default();
        let ok = GetIconInfo(icon, &mut info).is_ok();
        let dc = CreateCompatibleDC(None);
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: px,
                biHeight: -px,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let read = ok && GetDIBits(dc, info.hbmColor, 0, px as u32, Some(out.as_mut_ptr().cast()), &mut bmi, DIB_RGB_COLORS) == px;
        let _ = DeleteDC(dc);
        let _ = DeleteObject(info.hbmColor.into());
        let _ = DeleteObject(info.hbmMask.into());
        let _ = DestroyIcon(icon);
        read.then_some(())?;
    }
    premultiplied(&mut out);
    Some(out)
}

pub fn pixels(px: i32) -> Option<Rc<Vec<u32>>> {
    if let Some(hit) = CACHE.with(|c| c.borrow().as_ref().filter(|(k, _)| *k == px).map(|(_, p)| p.clone())) {
        return Some(hit);
    }
    let p = Rc::new(load(px)?);
    CACHE.with(|c| *c.borrow_mut() = Some((px, p.clone())));
    Some(p)
}
