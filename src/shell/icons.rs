use crate::{
    layered::Dib,
    render::{opaque_if_flat, shrink},
};
use std::{
    cell::RefCell,
    collections::HashMap,
    hash::Hash,
    rc::Rc,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};
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

struct Lru<K> {
    map: HashMap<K, (Scaled, u64)>,
    used: usize,
    clock: u64,
    budget: usize,
}

impl<K: Hash + Eq + Copy> Lru<K> {
    fn new(budget: usize) -> Self {
        Lru { map: HashMap::new(), used: 0, clock: 0, budget }
    }

    fn get(&mut self, k: &K) -> Option<Scaled> {
        self.clock += 1;
        let (v, t) = self.map.get_mut(k)?;
        *t = self.clock;
        Some(v.clone())
    }

    fn put(&mut self, k: K, v: Scaled) {
        self.clock += 1;
        self.used += v.len() * 4;
        if let Some((old, _)) = self.map.insert(k, (v, self.clock)) {
            self.used -= old.len() * 4;
        }
        if self.used <= self.budget {
            return;
        }
        let mut ages: Vec<(u64, K)> = self.map.iter().map(|(k, (_, t))| (*t, *k)).collect();
        ages.sort_unstable_by_key(|a| a.0);
        for (_, k) in ages {
            if self.used <= self.budget * 3 / 4 {
                break;
            }
            self.used -= self.map.remove(&k).map_or(0, |(v, _)| v.len() * 4);
        }
    }
}

thread_local! {
    static CACHE: RefCell<Lru<(i32, i32, i32)>> = RefCell::new(Lru::new(BUDGET));
}

fn query() -> Vec<(i32, i32)> {
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
}

fn lists() -> Option<&'static [(i32, i32)]> {
    static LISTS: OnceLock<Vec<(i32, i32)>> = OnceLock::new();
    static LOGGED: AtomicBool = AtomicBool::new(false);
    if let Some(l) = LISTS.get() {
        return Some(l);
    }
    let v = query();
    if v.is_empty() {
        if !LOGGED.swap(true, Ordering::Relaxed) {
            crate::report::log("system image lists unavailable, icons will be retried");
        }
        return None;
    }
    Some(LISTS.get_or_init(|| v))
}

fn pick(px: i32) -> Option<(i32, i32)> {
    let l = lists()?;
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
    if let Some(hit) = CACHE.with(|c| c.borrow_mut().get(&key)) {
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
    CACHE.with(|c| c.borrow_mut().put(key, small.clone()));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_least_recently_used_icons_go_first_and_the_budget_holds() {
        let mut c = Lru::new(40);
        let px = |n: usize| Rc::new(vec![0u32; n]);
        (0..5).for_each(|i| c.put(i, px(2)));
        assert!(c.get(&0).is_some() && c.used == 40);
        c.put(5, px(2));
        assert_eq!(c.used, 24);
        assert!([0, 4, 5].iter().all(|k| c.get(k).is_some()) && [1, 2, 3].iter().all(|k| c.get(k).is_none()));
        c.put(5, px(3));
        assert_eq!(c.used, 28);
    }
}
