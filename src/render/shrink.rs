pub fn shrink(src: &[u32], (sw, sh): (i32, i32), (dw, dh): (i32, i32)) -> Vec<u32> {
    let (fx, fy) = (sw as f32 / dw.max(1) as f32, sh as f32 / dh.max(1) as f32);
    let span = |d: i32, f: f32, n: i32| {
        let (a, b) = (d as f32 * f, (d + 1) as f32 * f);
        (a.floor() as i32..(b.ceil() as i32).min(n)).map(move |s| (s, (b.min((s + 1) as f32) - a.max(s as f32)).max(0.)))
    };
    let mut out = Vec::with_capacity((dw.max(0) * dh.max(0)) as usize);
    for dy in 0..dh {
        for dx in 0..dw {
            let (mut acc, mut total) = ([0f32; 4], 0f32);
            for (sy, wy) in span(dy, fy, sh) {
                for (sx, wx) in span(dx, fx, sw) {
                    let (p, w) = (src[(sy * sw + sx) as usize], wx * wy);
                    (0..4).for_each(|c| acc[c] += (p >> (8 * c) & 0xFF) as f32 * w);
                    total += w;
                }
            }
            out.push((0..4).fold(0, |px, c| px | ((acc[c] / total.max(f32::EPSILON)).round() as u32).min(255) << (8 * c)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_image_stays_plain_and_edges_are_kept() {
        assert_eq!(shrink(&[0xFF33_6699; 64 * 64], (64, 64), (24, 24)), vec![0xFF33_6699; 24 * 24]);
        let half: Vec<u32> = (0..4 * 4).map(|i| if i % 4 < 2 { 0xFF00_0000 } else { 0xFFFF_FFFF }).collect();
        assert_eq!(shrink(&half, (4, 4), (2, 2)), vec![0xFF00_0000, 0xFFFF_FFFF, 0xFF00_0000, 0xFFFF_FFFF]);
    }

    #[test]
    fn pixels_are_averaged_by_the_area_they_cover() {
        assert_eq!(shrink(&[0xFF00_0000, 0xFF00_00FF, 0xFF00_00FF, 0xFF00_0000], (2, 2), (1, 1)), vec![0xFF00_0080]);
        assert_eq!(shrink(&[0, 0x8080_8080], (2, 1), (1, 1)), vec![0x4040_4040], "transparent pixels lower the alpha too");
        let three: Vec<u32> = vec![0xFF00_0000, 0xFF00_00FF, 0xFF00_0000];
        assert_eq!(shrink(&three, (3, 1), (2, 1)), vec![0xFF00_0055, 0xFF00_0055], "the middle pixel is shared by both halves");
    }
}
