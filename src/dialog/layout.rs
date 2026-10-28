pub type Rect = (i32, i32, i32, i32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub width: i32,
    pub top: i32,
    pub pad: i32,
    pub gap: i32,
    pub line: i32,
    pub bar: i32,
    pub button: (i32, i32),
    pub max_body: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub body: Rect,
    pub status: Option<Rect>,
    pub bar: Option<Rect>,
    pub buttons: Vec<Rect>,
    pub height: i32,
}

pub fn layout(m: &Metrics, text: i32, progress: bool, buttons: usize) -> Layout {
    let (x0, x1) = (m.pad, m.width - m.pad);
    let y = m.top + m.gap;
    let body = (x0, y, x1, y + text.clamp(0, m.max_body));
    let mut y = body.3 + m.gap;
    let (status, bar) = if progress {
        let status = (x0, y, x1, y + m.line);
        let bar = (x0, status.3 + m.gap / 2, x1, status.3 + m.gap / 2 + m.bar);
        y = bar.3 + m.gap;
        (Some(status), Some(bar))
    } else {
        (None, None)
    };
    let (bw, bh) = m.button;
    let rects: Vec<Rect> = (0..buttons as i32).map(|i| (x1 - (i + 1) * bw - i * m.gap / 2, y, x1 - i * (bw + m.gap / 2), y + bh)).collect();
    if buttons > 0 {
        y += bh + m.gap;
    }
    Layout { body, status, bar, buttons: rects, height: y + m.pad / 2 }
}

pub fn fill((x0, y0, x1, y1): Rect, value: Option<f32>, phase: u32) -> Rect {
    let w = x1 - x0;
    match value {
        Some(v) => (x0, y0, x0 + (w as f32 * v.clamp(0., 1.)).round() as i32, y1),
        None => {
            let seg = w / 3;
            let start = x0 - seg + (phase as i32 * 6) % (w + seg);
            (start.max(x0), y0, (start + seg).min(x1), y1)
        }
    }
}

pub fn hit(rects: &[Rect], (x, y): (i32, i32)) -> Option<usize> {
    rects.iter().position(|&(x0, y0, x1, y1)| (x0..x1).contains(&x) && (y0..y1).contains(&y))
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: Metrics = Metrics { width: 460, top: 56, pad: 22, gap: 16, line: 20, bar: 8, button: (120, 36), max_body: 300 };

    #[test]
    fn stacks_body_progress_and_buttons() {
        let l = layout(&M, 100, true, 2);
        assert_eq!(l.body, (22, 72, 438, 172));
        assert_eq!((l.status, l.bar), (Some((22, 188, 438, 208)), Some((22, 216, 438, 224))));
        assert_eq!(l.buttons, vec![(318, 240, 438, 276), (190, 240, 310, 276)]);
        assert_eq!(l.height, 240 + 36 + 16 + 11);
    }

    #[test]
    fn long_text_is_capped_and_empty_rows_vanish() {
        let l = layout(&M, 5000, false, 0);
        assert_eq!((l.body.3 - l.body.1, l.status, l.bar, l.buttons.len()), (300, None, None, 0));
        assert_eq!(l.height, 72 + 300 + 16 + 11);
    }

    #[test]
    fn progress_fill_and_hits() {
        let r = (0, 0, 300, 8);
        assert_eq!((fill(r, Some(0.5), 0), fill(r, Some(2.), 0), fill(r, Some(-1.), 0)), ((0, 0, 150, 8), (0, 0, 300, 8), (0, 0, 0, 8)));
        let (a, b) = (fill(r, None, 0), fill(r, None, 40));
        assert!(a.2 - a.0 <= 100 && a != b && b.0 >= 0 && b.2 <= 300);
        assert_eq!(hit(&[(0, 0, 10, 10), (20, 0, 30, 10)], (25, 5)), Some(1));
        assert_eq!(hit(&[(0, 0, 10, 10)], (15, 5)), None);
    }
}
