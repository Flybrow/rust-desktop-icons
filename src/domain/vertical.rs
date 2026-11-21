use super::grid::Align;

pub fn ideographic(c: char) -> bool {
    matches!(
        c as u32,
        0x1100..=0x115F
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x2_0000..=0x3_FFFD
    )
}

pub fn has_ideographic(s: &str) -> bool {
    s.chars().any(ideographic)
}

pub fn stack(n: usize, line: i32, (from, to): (i32, i32), align: Align) -> Vec<i32> {
    let line = line.max(1);
    let extent = (to - from).max(0);
    let shown = n.min((extent / line) as usize);
    if shown == 0 {
        return vec![];
    }
    let total = shown as i32 * line;
    let start = from
        + match align {
            Align::Left => 0,
            Align::Center => (extent - total) / 2,
            Align::Right => extent - total,
        };
    (0..shown).map(|i| start + i as i32 * line).collect()
}

pub fn column(title: &str, line: i32, (from, to): (i32, i32), align: Align) -> Vec<(char, i32)> {
    let mut chars: Vec<char> = title.split_whitespace().collect::<Vec<_>>().join(" ").chars().collect();
    let room = ((to - from).max(0) / line.max(1)) as usize;
    if chars.len() > room {
        chars.truncate(room.saturating_sub(1));
        chars.extend((room > 0).then_some('…'));
    }
    let ys = stack(chars.len(), line, (from, to), align);
    chars.into_iter().zip(ys).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ideographic_characters_stack_upright() {
        assert!(ideographic('分') && ideographic('区') && ideographic('あ') && ideographic('ア') && ideographic('한'));
        assert!(ideographic('，') && ideographic('Ａ'));
        assert!(!ideographic('A') && !ideographic('7') && !ideographic('-') && !ideographic(' ') && !ideographic('🎮'));
        assert!(has_ideographic("分区") && has_ideographic("RDI 区域") && !has_ideographic("Tools 2026"));
    }

    #[test]
    fn stacked_cells_follow_the_alignment() {
        assert_eq!(stack(2, 20, (0, 100), Align::Left), vec![0, 20]);
        assert_eq!(stack(2, 20, (0, 100), Align::Center), vec![30, 50]);
        assert_eq!(stack(2, 20, (0, 100), Align::Right), vec![60, 80]);
        assert_eq!(stack(10, 20, (0, 100), Align::Left), vec![0, 20, 40, 60, 80]);
        assert_eq!(stack(3, 40, (10, 50), Align::Left), vec![10]);
        assert_eq!(stack(0, 20, (0, 100), Align::Center), Vec::<i32>::new());
        assert_eq!(stack(1, 0, (0, 100), Align::Left), vec![0]);
    }

    #[test]
    fn columns_keep_one_space_and_end_with_an_ellipsis_when_cut() {
        let chars = |v: Vec<(char, i32)>| v.into_iter().map(|(c, _)| c).collect::<String>();
        assert_eq!(column("  文件   夹 ", 10, (0, 100), Align::Left), vec![('文', 0), ('件', 10), (' ', 20), ('夹', 30)]);
        assert_eq!(chars(column("分区一二三四", 10, (0, 30), Align::Left)), "分区…");
        assert_eq!(chars(column("分区", 10, (0, 20), Align::Left)), "分区");
        assert_eq!(column("分区", 10, (0, 25), Align::Right), vec![('分', 5), ('区', 15)]);
        assert_eq!(chars(column("分区", 10, (0, 10), Align::Left)), "…");
        assert!(column("分区", 10, (0, 9), Align::Left).is_empty());
        assert!(column("   ", 10, (0, 100), Align::Center).is_empty());
    }
}
