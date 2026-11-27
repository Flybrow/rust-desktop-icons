use super::{blur::pass, lerp, premul};

pub const MARGIN: i32 = 64;
const SCALE: i32 = 4;
const RADIUS: usize = 2;

pub type Cached = ((u32, u8), (i32, i32, i32, i32), std::rc::Rc<Frost>);

pub struct Frost {
    w: i32,
    h: i32,
    px: Vec<u32>,
}

fn shrink(src: &[u32], (w, h): (i32, i32)) -> ([Vec<u8>; 3], i32, i32) {
    let (dw, dh) = ((w + SCALE - 1) / SCALE, (h + SCALE - 1) / SCALE);
    let mut out = [vec![0u8; (dw * dh) as usize], vec![0u8; (dw * dh) as usize], vec![0u8; (dw * dh) as usize]];
    for dy in 0..dh {
        for dx in 0..dw {
            let (mut sum, mut n) = ([0u32; 3], 0u32);
            for y in dy * SCALE..((dy + 1) * SCALE).min(h) {
                for x in dx * SCALE..((dx + 1) * SCALE).min(w) {
                    let p = src[(y * w + x) as usize];
                    (0..3).for_each(|c| sum[c] += p >> (8 * c) & 0xFF);
                    n += 1;
                }
            }
            (0..3).for_each(|c| out[c][(dy * dw + dx) as usize] = (sum[c] / n.max(1)) as u8);
        }
    }
    (out, dw, dh)
}

fn soften(ch: &mut [u8], (w, h): (i32, i32)) {
    let (w, h) = (w as usize, h as usize);
    let mut tmp = vec![0u8; ch.len()];
    for _ in 0..2 {
        pass(ch, &mut tmp, (h, w, w, 1), RADIUS);
        pass(&tmp, ch, (w, h, 1, w), RADIUS);
    }
}

fn vivid([b, g, r]: [u8; 3]) -> u32 {
    let gray = (r as i32 * 299 + g as i32 * 587 + b as i32 * 114) / 1000;
    let push = |c: u8| (gray + (c as i32 - gray) * 6 / 5).clamp(0, 255) as u32;
    0xFF00_0000 | push(r) << 16 | push(g) << 8 | push(b)
}

fn luma(p: u32) -> i32 {
    ((p >> 16 & 0xFF) * 299 + (p >> 8 & 0xFF) * 587 + (p & 0xFF) * 114) as i32 / 1000
}

fn legible(p: u32, tint: u32) -> u32 {
    let (y, ty) = (luma(p), luma(tint));
    let limit = if ty > 140 { 185 } else { 105 };
    if (ty > 140) == (y >= limit) || y == ty {
        return p;
    }
    lerp(p, tint, ((y - limit) * 255 / (y - ty)).clamp(0, 255) as u32)
}

pub fn strength(alpha: u8, light: bool) -> u8 {
    let (a, base, low) = (alpha as i32, if light { 119 } else { 65 }, 64);
    let v = if a <= 217 { (a - low).max(0) * base / (217 - low) } else { base + (a - 217) * (215 - base) / (255 - 217) };
    v.clamp(0, 215) as u8
}

pub fn build(wallpaper: &[u32], size: (i32, i32), tint: u32, strength: u8) -> Frost {
    let tint = premul(tint, 255);
    frosted(wallpaper, size, |p| legible(lerp(p, tint, strength as u32), tint))
}

fn frosted(wallpaper: &[u32], (w, h): (i32, i32), paint: impl Fn(u32) -> u32) -> Frost {
    let (sw, sh) = (w + 2 * MARGIN, h + 2 * MARGIN);
    let (mut ch, dw, dh) = shrink(wallpaper, (sw, sh));
    ch.iter_mut().for_each(|c| soften(c, (dw, dh)));
    let (m, cw, chh) = (MARGIN / SCALE, (w + SCALE - 1) / SCALE + 1, (h + SCALE - 1) / SCALE + 1);
    let mut px = Vec::with_capacity((cw * chh) as usize);
    for y in 0..chh {
        for x in 0..cw {
            let i = ((y + m).min(dh - 1) * dw + (x + m).min(dw - 1)) as usize;
            px.push(paint(vivid([ch[0][i], ch[1][i], ch[2][i]])) | 0xFF00_0000);
        }
    }
    Frost { w: cw, h: chh, px }
}

impl Frost {
    pub fn at(&self, x: i32, y: i32) -> u32 {
        let axis = |v: i32, n: i32| {
            let f = ((v * 256 + 128) / SCALE - 128).max(0);
            let i = (f >> 8).min(n - 1);
            (i, (i + 1).min(n - 1), if i == n - 1 { 0 } else { (f & 0xFF) as u32 })
        };
        let ((x0, x1, kx), (y0, y1, ky)) = (axis(x, self.w), axis(y, self.h));
        let p = |x: i32, y: i32| self.px[(y * self.w + x) as usize];
        let (a, b, c, d) = (p(x0, y0), p(x1, y0), p(x0, y1), p(x1, y1));
        let w = [(256 - kx) * (256 - ky), kx * (256 - ky), (256 - kx) * ky, kx * ky];
        (0..3).fold(0xFF00_0000, |acc, i| {
            let ch = |q: u32| q >> (8 * i) & 0xFF;
            acc | ((ch(a) * w[0] + ch(b) * w[1] + ch(c) * w[2] + ch(d) * w[3] + 32768) >> 16) << (8 * i)
        })
    }

    #[cfg(test)]
    fn bytes(&self) -> usize {
        self.px.len() * 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wall(w: i32, h: i32, f: impl Fn(i32, i32) -> u32) -> Vec<u32> {
        (0..(h + 2 * MARGIN)).flat_map(|y| (0..(w + 2 * MARGIN)).map(move |x| (x, y))).map(|(x, y)| f(x, y)).collect()
    }

    #[test]
    fn a_plain_wallpaper_gives_the_tinted_plain_color_and_is_opaque() {
        let m = build(&wall(100, 60, |_, _| 0x00_40_40_40), (100, 60), 0x00_0000, 128);
        let p = m.at(50, 30);
        assert_eq!(p >> 24, 0xFF);
        assert!((0x1F..=0x21).contains(&(p & 0xFF)), "{p:08x}");
        assert_eq!(m.at(0, 0), m.at(99, 59));
        let gray = build(&wall(40, 40, |_, _| 0x00_50_50_50), (40, 40), 0, 0);
        assert_eq!(gray.at(10, 10), 0xFF_50_50_50);
    }

    #[test]
    fn opacity_ranges_from_bare_wallpaper_to_the_fence_color() {
        assert_eq!((strength(217, false), strength(217, true)), (65, 119));
        assert_eq!((strength(64, false), strength(255, false), strength(255, true)), (0, 215, 215));
        assert!(strength(76, false) < 8 && strength(76, true) < 12);
        let steps: Vec<u8> = crate::domain::Look::PERCENTS.map(|p| strength(crate::domain::Look::alpha_for(p), false)).to_vec();
        assert!(steps.windows(2).all(|w| w[0] < w[1]), "{steps:?}");
    }

    #[test]
    fn text_stays_readable_on_any_wallpaper() {
        let dark_on_white = build(&wall(40, 40, |_, _| 0x00_FF_FF_E0), (40, 40), 0x20_2020, 60);
        assert!(luma(dark_on_white.at(10, 10)) <= 106, "{:08x}", dark_on_white.at(10, 10));
        let dark_on_dark = build(&wall(40, 40, |_, _| 0x00_30_10_40), (40, 40), 0x20_2020, 60);
        assert!(luma(dark_on_dark.at(10, 10)) < 60, "a dark wallpaper keeps its own colors");
        let light_on_black = build(&wall(40, 40, |_, _| 0), (40, 40), 0xF3_F3F3, 100);
        assert!(luma(light_on_black.at(10, 10)) >= 184, "{:08x}", light_on_black.at(10, 10));
    }

    #[test]
    fn colors_get_more_vivid_and_edges_are_blurred() {
        let m = frosted(&wall(40, 40, |_, _| 0x00_80_60_40), (40, 40), |p| p);
        let p = m.at(10, 10);
        assert!((p >> 16 & 0xFF) > 0x80 && (p & 0xFF) < 0x40, "{p:08x}");
        let m = frosted(&wall(320, 240, |x, _| if x < 64 + 160 { 0 } else { 0xFF_FFFF }), (320, 240), |p| p);
        let (left, mid, right) = (m.at(10, 120) & 0xFF, m.at(160, 120) & 0xFF, m.at(310, 120) & 0xFF);
        assert!(left < 10 && right > 245 && (40..215).contains(&mid), "{left} {mid} {right}");
        assert!(m.bytes() < 320 * 240 * 4 / 12);
    }

    #[test]
    fn reads_outside_the_footprint_stay_on_its_edge() {
        let m = frosted(&wall(40, 30, |x, y| (x * 3 + y) as u32), (40, 30), |p| p);
        assert_eq!((m.at(-50, -50), m.at(-1, 0)), (m.at(0, 0), m.at(0, 0)));
        assert_eq!((m.at(500, 10), m.at(10, 400)), (m.at(9999, 10), m.at(10, 9999)));
    }
}
