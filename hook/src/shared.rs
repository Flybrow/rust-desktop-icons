use std::collections::HashSet;

pub const MESSAGE: &str = "RustDesktopIcons.Hide";
pub const REPLY: &str = "RustDesktopIcons.Hidden";
pub const HEAD: usize = 17;
pub const CHARS: usize = 1 << 20;
pub const BYTES: usize = HEAD * 4 + CHARS * 2;

pub const SEQ: usize = 0;
pub const LEN: usize = 1;
pub const PASSES: usize = 2;
pub const REMOVED: usize = 3;
pub const ADDED: usize = 4;
pub const INSERTS: usize = 5;
pub const REFRESHES: usize = 6;
pub const ERRORS: usize = 7;
pub const ITEMS: usize = 8;
pub const MICROS: usize = 9;
pub const HIDDEN: usize = 10;
pub const FILTERED: usize = 11;
pub const INSTALLS: usize = 12;
pub const STAGE: usize = 13;
pub const HR: usize = 14;
pub const SWEEPS: usize = 15;
pub const TRAY: usize = 16;

pub const DONE: u32 = 5;
pub const NOT_PINNED: u32 = 9;

pub fn name(pid: u32) -> String {
    format!("Local\\RustDesktopIcons.Hide.{pid}")
}

pub fn key(path: &str) -> String {
    path.to_lowercase()
}

pub fn encode<S: AsRef<str>>(paths: &[S]) -> (Vec<u16>, usize) {
    let mut out: Vec<u16> = vec![];
    for (n, p) in paths.iter().enumerate() {
        let one: Vec<u16> = key(p.as_ref()).encode_utf16().collect();
        if out.len() + usize::from(n > 0) + one.len() > CHARS {
            return (out, n);
        }
        if n > 0 {
            out.push(b'\n' as u16);
        }
        out.extend(one);
    }
    (out, paths.len())
}

pub fn decode(data: &[u16]) -> HashSet<String> {
    String::from_utf16_lossy(data).split('\n').filter(|s| !s.is_empty()).map(String::from).collect()
}

pub fn changes<'a>(hidden: &'a [String], list: &'a HashSet<String>) -> (Vec<&'a String>, Vec<&'a String>) {
    let show = hidden.iter().filter(|p| !list.contains(*p)).collect();
    let was: HashSet<&String> = hidden.iter().collect();
    let mut hide: Vec<&String> = list.iter().filter(|p| !was.contains(p)).collect();
    hide.sort();
    (show, hide)
}

pub fn refused(alive: bool, list: Option<&HashSet<String>>, path: impl FnOnce() -> Option<String>) -> bool {
    alive && list.is_some_and(|l| !l.is_empty() && path().is_some_and(|p| l.contains(&key(&p))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_lowercases_and_skips_empty() {
        let (data, n) = encode(&["C:\\Users\\A\\Desktop\\Été.txt", "c:\\x\\b.lnk"]);
        let set = decode(&data);
        assert_eq!((set.len(), n), (2, 2));
        assert!(set.contains("c:\\users\\a\\desktop\\été.txt") && set.contains("c:\\x\\b.lnk"));
        assert_eq!(encode::<&str>(&[]), (vec![], 0));
    }

    #[test]
    fn encode_never_cuts_a_path() {
        assert_eq!(encode(&["a".repeat(CHARS + 10)]), (vec![], 0));
        let half = "b".repeat(CHARS / 2);
        let (data, n) = encode(&[half.clone(), half.clone(), "c".into()]);
        assert_eq!((n, data.len()), (1, CHARS / 2));
        let (data, n) = encode(&["d".repeat(CHARS / 2 - 1), half, "e".into()]);
        assert_eq!((n, data.len(), decode(&data).len()), (2, CHARS, 2));
    }

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn changes_show_unpinned_and_hide_new_pins() {
        let hidden = vec!["a".to_string(), "b".to_string()];
        let list = set(&["b", "d", "c"]);
        let (show, hide) = changes(&hidden, &list);
        assert_eq!(show, vec!["a"]);
        assert_eq!(hide, vec!["c", "d"]);
        let empty = set(&[]);
        let (show, hide) = changes(&[], &empty);
        assert!(show.is_empty() && hide.is_empty());
    }

    #[test]
    fn refused_only_pinned_items_while_rdi_lives() {
        let list = set(&["c:/d/a.txt"]);
        assert!(refused(true, Some(&list), || Some("C:/D/A.txt".into())));
        assert!(!refused(true, Some(&list), || Some("c:/d/b.txt".into())));
        assert!(!refused(true, Some(&list), || None));
        assert!(!refused(false, Some(&list), || panic!("no name lookup once RDI is gone")));
        assert!(!refused(true, None, || panic!("no name lookup without a list")));
        assert!(!refused(true, Some(&set(&[])), || panic!("no name lookup for an empty list")));
    }
}
