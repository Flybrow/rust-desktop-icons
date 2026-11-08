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

pub fn fit((w, h): (i32, i32), size: i32) -> (i32, i32) {
    let m = w.max(h).max(1);
    ((w * size / m).max(1), (h * size / m).max(1))
}

pub fn blit_fit(dst: &mut [u32], dw: i32, src: &[u32], (sw, sh): (i32, i32), (x, y): (i32, i32), size: i32) {
    let (fw, fh) = fit((sw, sh), size);
    let scaled: Vec<u32> = (0..fh).flat_map(|j| (0..fw).map(move |i| src[((j * sh / fh) * sw + i * sw / fw) as usize])).collect();
    blit(dst, dw, &scaled, fw, (x + (size - fw) / 2, y + (size - fh) / 2));
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
    fn thumbnails_fit_their_box() {
        assert_eq!((fit((200, 100), 48), fit((50, 100), 48), fit((48, 48), 48)), ((48, 24), (24, 48), (48, 48)));
        let mut dst = vec![0u32; 8 * 8];
        blit_fit(&mut dst, 8, &[1, 2, 3, 4], (2, 2), (2, 2), 4);
        assert_eq!(&dst[2 * 8 + 2..2 * 8 + 6], &[1, 1, 2, 2]);
        assert_eq!(&dst[5 * 8 + 2..5 * 8 + 6], &[3, 3, 4, 4]);
        let mut dst = vec![0u32; 8 * 8];
        blit_fit(&mut dst, 8, &[7, 8], (2, 1), (0, 0), 4);
        assert_eq!((dst[8], dst[2 * 8 + 3], dst[0]), (7, 8, 0));
    }

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
