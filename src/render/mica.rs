use super::{blur::pass, lerp, premul};

pub const MARGIN: i32 = 64;
const SCALE: i32 = 8;
const RADIUS: usize = 3;

fn luma(argb: u32) -> u32 {
    ((argb >> 16 & 0xFF) * 299 + (argb >> 8 & 0xFF) * 587 + (argb & 0xFF) * 114) / 1000
}

#[derive(Clone)]
pub struct Mica {
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

pub fn build(wallpaper: &[u32], (w, h): (i32, i32), tint: u32, strength: u8) -> Mica {
    let (sw, sh) = (w + 2 * MARGIN, h + 2 * MARGIN);
    let (mut ch, dw, dh) = shrink(wallpaper, (sw, sh));
    ch.iter_mut().for_each(|c| soften(c, (dw, dh)));
    let (m, cw, chh) = (MARGIN / SCALE, (w + SCALE - 1) / SCALE + 1, (h + SCALE - 1) / SCALE + 1);
    let tint = premul(tint, 255);
    let k = strength as u32 + (255 - strength as u32) * strength as u32 * 3 / (5 * 255);
    let lt = luma(tint);
    let mut px = Vec::with_capacity((cw * chh) as usize);
    for y in 0..chh {
        for x in 0..cw {
            let i = ((y + m).min(dh - 1) * dw + (x + m).min(dw - 1)) as usize;
            let rgb = [ch[2][i] as u32, ch[1][i] as u32, ch[0][i] as u32];
            let l = luma(0xFF00_0000 | rgb[0] << 16 | rgb[1] << 8 | rgb[2]);
            let target = (l * (255 - k) + lt * k) / 255;
            let fit = |v: u32| match target <= l {
                true => (v * target).checked_div(l).unwrap_or(0),
                false => v + (255 - v) * (target - l) / (255 - l).max(1),
            };
            let fitted = rgb.map(fit);
            let mid = (fitted[0] * 299 + fitted[1] * 587 + fitted[2] * 114) / 1000;
            let vivid = fitted.map(|v| (mid as i32 + (v as i32 - mid as i32) * (510 + k as i32) / 510).clamp(0, 255) as u32);
            let base = 0xFF00_0000 | vivid[0] << 16 | vivid[1] << 8 | vivid[2];
            px.push(lerp(base, tint, k / 8) | 0xFF00_0000);
        }
    }
    Mica { w: cw, h: chh, px }
}

impl Mica {
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

    pub fn average(&self) -> u32 {
        let n = self.px.len().max(1) as u64;
        let sum = self
            .px
            .iter()
            .fold([0u64; 3], |s, p| [s[0] + (p >> 16 & 0xFF) as u64, s[1] + (p >> 8 & 0xFF) as u64, s[2] + (p & 0xFF) as u64]);
        (sum[0] / n) as u32 | ((sum[1] / n) as u32) << 8 | ((sum[2] / n) as u32) << 16
    }

    #[cfg(test)]
    pub fn bytes(&self) -> usize {
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
        let m = build(&wall(100, 60, |_, _| 0x00_FF_FF_FF), (100, 60), 0x00_0000, 128);
        let p = m.at(50, 30);
        assert_eq!(p >> 24, 0xFF);
        assert!((0x48..=0x58).contains(&(p & 0xFF)), "{p:08x}");
        assert_eq!(m.at(0, 0), m.at(99, 59));
        let full = build(&wall(40, 40, |_, _| 0x00_12_34_56), (40, 40), 0x00_0000, 0);
        assert_eq!(full.at(10, 10), 0xFF_12_34_56);
        assert_eq!(full.average(), 0x56_34_12, "average is a COLORREF like the look colors");
    }

    #[test]
    fn the_tint_sets_the_brightness_but_the_wallpaper_keeps_its_hue() {
        let green = build(&wall(40, 40, |_, _| 0x00_20_C0_40), (40, 40), 0x20_2020, 217).at(10, 10);
        let (r, g, b) = (green >> 16 & 0xFF, green >> 8 & 0xFF, green & 0xFF);
        assert!(g > r + 10 && g > b + 10 && g < 0x60, "dark but still green: {green:08x}");
        let pink = build(&wall(40, 40, |_, _| 0x00_F0_A0_C0), (40, 40), 0xF3_F3F3, 217).at(10, 10);
        assert!(luma(pink) > 0xD0 && (pink >> 16 & 0xFF) > (pink >> 8 & 0xFF), "light and still pinkish: {pink:08x}");
    }

    #[test]
    fn edges_are_blurred_and_the_cache_is_small() {
        let m = build(&wall(320, 240, |x, _| if x < 64 + 160 { 0 } else { 0xFF_FFFF }), (320, 240), 0, 0);
        let (left, mid, right) = (m.at(10, 120) & 0xFF, m.at(160, 120) & 0xFF, m.at(310, 120) & 0xFF);
        assert!(left < 10 && right > 245 && (40..215).contains(&mid), "{left} {mid} {right}");
        assert!(m.bytes() < 320 * 240 * 4 / 40);
    }
}
