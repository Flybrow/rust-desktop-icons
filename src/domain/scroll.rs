pub fn thumb(view: i32, total: i32, scroll: i32, min: i32) -> Option<(i32, i32)> {
    if total <= view || view <= 0 {
        return None;
    }
    let len = (view * view / total).max(min).min(view);
    let max = total - view;
    Some((scroll.clamp(0, max) * (view - len) / max, len))
}

#[cfg(test)]
mod tests {
    use super::thumb;

    #[test]
    fn no_thumb_when_everything_fits() {
        assert_eq!(thumb(500, 500, 0, 24), None);
        assert_eq!(thumb(500, 300, 0, 24), None);
        assert_eq!(thumb(0, 300, 0, 24), None);
    }

    #[test]
    fn thumb_is_proportional_and_travels_the_whole_track() {
        assert_eq!(thumb(400, 800, 0, 24), Some((0, 200)));
        assert_eq!(thumb(400, 800, 400, 24), Some((200, 200)));
        assert_eq!(thumb(400, 800, 200, 24), Some((100, 200)));
    }

    #[test]
    fn thumb_keeps_a_minimum_length_and_stays_inside() {
        let (top, len) = thumb(100, 10_000, 9_900, 24).unwrap();
        assert_eq!((top + len, len), (100, 24));
        assert_eq!(thumb(100, 10_000, 99_999, 24), Some((76, 24)));
        assert_eq!(thumb(100, 10_000, -5, 24), Some((0, 24)));
    }
}
