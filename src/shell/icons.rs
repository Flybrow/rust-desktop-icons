use crate::{
    layered::Dib,
    render::{opaque_if_flat, shrink},
};
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::OnceLock};
use windows::{
    Win32::{
        Foundation::COLORREF,
        Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, GdiFlush, HDC, SelectObject,
            SetDIBitsToDevice,
        },
        UI::{
            Controls::{CLR_NONE, HIMAGELIST, IImageList, ILD_TRANSPARENT, IMAGELISTDRAWPARAMS},
            Shell::{SHGetImageList, SHIL_EXTRALARGE, SHIL_JUMBO, SHIL_LARGE, SHIL_SMALL},
        },
    },
    core::Interface,
};

const BUDGET: usize = 6 << 20;

type Scaled = Rc<Vec<u32>>;
type Cache = (HashMap<(i32, i32, i32), Scaled>, usize);

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new((HashMap::new(), 0));
}

fn lists() -> &'static [(i32, i32)] {
    static LISTS: OnceLock<Vec<(i32, i32)>> = OnceLock::new();
    LISTS.get_or_init(|| {
        let mut v: Vec<(i32, i32)> = [SHIL_SMALL, SHIL_LARGE, SHIL_EXTRALARGE, SHIL_JUMBO]
            .into_iter()
            .filter_map(|l| {
                let list = unsafe { SHGetImageList::<IImageList>(l as i32) }.ok()?;
                let (mut w, mut h) = (0, 0);
                unsafe { list.GetIconSize(&mut w, &mut h) }.ok()?;
                Some((l as i32, w))
            })
            .collect();
        v.sort_by_key(|&(_, n)| n);
        v
    })
}

fn pick(px: i32) -> Option<(i32, i32)> {
    let l = lists();
    l.iter().find(|&&(_, n)| n >= px).or(l.last()).copied()
}

fn draw(dc: HDC, list: i32, index: i32, (x, y): (i32, i32), n: i32) {
    unsafe {
        if let Ok(l) = SHGetImageList::<IImageList>(list) {
            let p = IMAGELISTDRAWPARAMS {
                cbSize: size_of::<IMAGELISTDRAWPARAMS>() as u32,
                himl: HIMAGELIST(l.as_raw() as _),
                i: index,
                hdcDst: dc,
                x,
                y,
                cx: n,
                cy: n,
                rgbBk: COLORREF(CLR_NONE as u32),
                fStyle: ILD_TRANSPARENT.0,
                ..Default::default()
            };
            let _ = l.Draw(&p);
        }
    }
}

fn scaled(list: i32, index: i32, (n, px): (i32, i32)) -> Option<Scaled> {
    let key = (list, index, px);
    if let Some(hit) = CACHE.with(|c| c.borrow().0.get(&key).cloned()) {
        return Some(hit);
    }
    let mut big = Dib::new(n, n)?;
    unsafe {
        let dc = CreateCompatibleDC(None);
        let old = SelectObject(dc, big.bmp.into());
        draw(dc, list, index, (0, 0), n);
        let _ = GdiFlush();
        SelectObject(dc, old);
        let _ = DeleteDC(dc);
    }
    opaque_if_flat(big.px(), n, (0, 0, n, n));
    let small = Rc::new(shrink(big.px(), (n, n), (px, px)));
    CACHE.with(|c| {
        let (map, used) = &mut *c.borrow_mut();
        if *used + small.len() * 4 > BUDGET {
            map.clear();
            *used = 0;
        }
        *used += small.len() * 4;
        map.insert(key, small.clone());
    });
    Some(small)
}

pub fn draw_icon(dc: HDC, index: i32, (x, y): (i32, i32), px: i32) {
    let Some((list, n)) = pick(px) else { return };
    if n == px {
        return draw(dc, list, index, (x, y), px);
    }
    let Some(pixels) = scaled(list, index, (n, px)) else { return };
    let info = BITMAPINFO {
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
    unsafe { SetDIBitsToDevice(dc, x, y, px as u32, px as u32, 0, 0, 0, px as u32, pixels.as_ptr().cast(), &info, DIB_RGB_COLORS) };
}
