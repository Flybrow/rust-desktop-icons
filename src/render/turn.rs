pub fn rotate_cw(src: &[u32], w: i32, h: i32) -> Vec<u32> {
    let (w, h) = (w as usize, h as usize);
    let mut out = vec![0; w * h];
    for y in 0..h {
        for x in 0..w {
            out[x * h + (h - 1 - y)] = src[y * w + x];
        }
    }
    out
}

pub fn blit(dst: &mut [u32], dw: i32, src: &[u32], sw: i32, (dx, dy): (i32, i32)) {
    let (dh, sh) = (dst.len() as i32 / dw.max(1), src.len() as i32 / sw.max(1));
    for y in dy.max(0)..(dy + sh).min(dh) {
        let (x0, x1) = (dx.max(0), (dx + sw).min(dw));
        if x0 >= x1 {
            return;
        }
        let s = ((y - dy) * sw + x0 - dx) as usize;
        let d = (y * dw + x0) as usize;
        dst[d..d + (x1 - x0) as usize].copy_from_slice(&src[s..s + (x1 - x0) as usize]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_turn_reads_top_to_bottom() {
        let src = [1, 2, 3, 4, 5, 6];
        assert_eq!(rotate_cw(&src, 3, 2), vec![4, 1, 5, 2, 6, 3]);
        assert_eq!(rotate_cw(&rotate_cw(&rotate_cw(&rotate_cw(&src, 3, 2), 2, 3), 3, 2), 2, 3), src.to_vec());
    }

    #[test]
    fn blit_overwrites_and_clips() {
        let mut dst = vec![9; 12];
        blit(&mut dst, 4, &[1, 2, 3, 4], 2, (3, 2));
        assert_eq!(dst, vec![9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 1]);
        let mut dst = vec![0; 4];
        blit(&mut dst, 2, &[7, 7, 7, 7], 2, (-1, -1));
        assert_eq!(dst, vec![7, 0, 0, 0]);
    }
}
