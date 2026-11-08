fn rgb(c: u32) -> [i32; 3] {
    [(c & 0xFF) as i32, (c >> 8 & 0xFF) as i32, (c >> 16 & 0xFF) as i32]
}

pub fn shade(c: u32, amount: i32) -> u32 {
    rgb(c).iter().enumerate().fold(0, |acc, (i, v)| acc | ((v + amount).clamp(0, 255) as u32) << (8 * i))
}

fn light(c: u32) -> bool {
    let [r, g, b] = rgb(c);
    r * 299 + g * 587 + b * 114 > 140_000
}

pub fn contrast(c: u32) -> u32 {
    if light(c) { 0x20_2020 } else { 0xF5_F5F5 }
}

pub fn text_on(base: u32) -> (u32, u32) {
    let fg = contrast(base);
    (fg, if fg > 0x80_8080 { 0 } else { 0xFF_FFFF })
}

pub fn average(argb: &[u32]) -> Option<u32> {
    let n = argb.len() as u64;
    let sum = argb.iter().fold([0u64; 3], |s, p| [s[0] + (p >> 16 & 0xFF) as u64, s[1] + (p >> 8 & 0xFF) as u64, s[2] + (p & 0xFF) as u64]);
    (n > 0).then(|| (sum[0] / n) as u32 | ((sum[1] / n) as u32) << 8 | ((sum[2] / n) as u32) << 16)
}

pub fn from_wallpaper(avg: u32) -> u32 {
    let c = rgb(avg);
    let top = c.iter().copied().max().unwrap_or(0).max(1);
    c.iter().enumerate().fold(0, |acc, (i, v)| acc | ((v * 72 / top) as u32) << (8 * i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors() {
        assert_eq!(shade(0x10_20F0, 20), 0x24_34FF);
        assert_eq!(shade(0x05_0505, -10), 0);
        assert_eq!(contrast(0xFF_FFFF), 0x20_2020);
        assert_eq!(contrast(0), 0xF5_F5F5);
        assert_eq!((text_on(0), text_on(0xFF_FFFF)), ((0xF5_F5F5, 0), (0x20_2020, 0xFF_FFFF)));
        assert_eq!(average(&[0xFF_10_20_30, 0xFF_30_40_50]), Some(0x40_30_20));
        assert_eq!(average(&[]), None);
        assert_eq!((from_wallpaper(0xF0_F0F0), from_wallpaper(0x10_1010), from_wallpaper(0x8A_5A3A)), (0x48_4848, 0x48_4848, 0x48_2E1E));
        assert_eq!(contrast(from_wallpaper(0xFF_FFFF)), 0xF5_F5F5);
    }
}
