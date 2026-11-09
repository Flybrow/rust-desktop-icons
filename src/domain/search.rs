use super::Fence;

pub const LIMIT: usize = 8;

pub fn place(f: &Fence, tab: u64) -> String {
    match f.tabs.iter().find(|t| t.id == tab) {
        Some(t) if f.tabs.len() > 1 => format!("{} › {}", f.tabs[0].title, t.title),
        _ => f.active().title.clone(),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Click {
    pub time: u32,
    pub at: (i32, i32),
}

pub fn double(prev: Option<Click>, now: Click, max_ms: u32, (cx, cy): (i32, i32)) -> bool {
    prev.is_some_and(|p| {
        now.time.wrapping_sub(p.time) <= max_ms && (p.at.0 - now.at.0).abs() <= cx / 2 && (p.at.1 - now.at.1).abs() <= cy / 2
    })
}

fn fold(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' => 'i',
        'ñ' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
        'ù' | 'ú' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        c => c,
    }
}

pub fn normalize(s: &str) -> String {
    s.chars().flat_map(char::to_lowercase).map(fold).collect()
}

fn score(query: &str, name: &str) -> Option<u8> {
    let name = normalize(name);
    let at = name.find(query)?;
    let word = at == 0 || name[..at].ends_with(|c: char| !c.is_alphanumeric());
    Some(if at == 0 {
        0
    } else if word {
        1
    } else {
        2
    })
}

pub fn rank(query: &str, names: &[String]) -> Vec<usize> {
    let q = normalize(query.trim());
    if q.is_empty() {
        return vec![];
    }
    let mut hits: Vec<(u8, usize)> = names.iter().enumerate().filter_map(|(i, n)| score(&q, n).map(|s| (s, i))).collect();
    hits.sort_by_key(|&(s, i)| (s, names[i].chars().count(), normalize(&names[i])));
    hits.into_iter().map(|(_, i)| i).take(LIMIT).collect()
}

pub fn step(sel: usize, n: usize, down: bool) -> usize {
    match (n, down) {
        (0, _) => 0,
        (_, true) => (sel + 1) % n,
        (_, false) => (sel + n - 1) % n,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranking() {
        let names: Vec<String> =
            ["Mon document.docx", "Documents", "Photo été.jpg", "Steam", "readme.md", "Doc"].map(String::from).to_vec();
        assert_eq!(rank("doc", &names), vec![5, 1, 0]);
        assert_eq!(rank("ETE", &names), vec![2]);
        assert_eq!(rank("e", &names), vec![2, 3, 1, 4, 0]);
        assert_eq!(rank("  ", &names), Vec::<usize>::new());
        assert_eq!(rank("zzz", &names), Vec::<usize>::new());
        let many: Vec<String> = (0..20).map(|i| format!("a{i}")).collect();
        assert_eq!(rank("a", &many).len(), LIMIT);
    }

    #[test]
    fn places_and_double_clicks() {
        let mut f = Fence::new(1, "Jeux", (0, 0, 100, 100), crate::domain::Look::default());
        assert_eq!(place(&f, 1), "Jeux");
        f.tabs.push(crate::domain::Tab { id: 7, title: "Steam".into(), ..Default::default() });
        assert_eq!((place(&f, 7), place(&f, 99)), ("Jeux › Steam".to_string(), "Jeux".to_string()));
        let c = |time, at| Click { time, at };
        assert!(double(Some(c(1000, (10, 10))), c(1400, (12, 9)), 500, (4, 4)));
        assert!(!double(Some(c(1000, (10, 10))), c(1600, (10, 10)), 500, (4, 4)));
        assert!(!double(Some(c(1000, (10, 10))), c(1100, (20, 10)), 500, (4, 4)));
        assert!(!double(None, c(1100, (10, 10)), 500, (4, 4)));
        assert!(double(Some(c(u32::MAX - 10, (0, 0))), c(50, (0, 0)), 500, (4, 4)));
    }

    #[test]
    fn selection_wraps() {
        assert_eq!((step(0, 3, true), step(2, 3, true), step(0, 3, false), step(0, 0, true)), (1, 0, 2, 0));
    }

    #[test]
    fn a_double_click_stays_within_half_a_cell() {
        let c = |time, at| Click { time, at };
        assert!(double(Some(c(0, (0, 0))), c(100, (40, 30)), 500, (80, 60)));
        assert!(!double(Some(c(0, (0, 0))), c(100, (41, 0)), 500, (80, 60)) && !double(Some(c(0, (0, 0))), c(100, (0, 31)), 500, (80, 60)));
    }
}
