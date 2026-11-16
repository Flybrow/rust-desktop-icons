use std::collections::HashSet;

pub const MESSAGE: &str = "RustDesktopIcons.Hide";
pub const HEAD: usize = 16;
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

pub const DONE: u32 = 5;
pub const NOT_PINNED: u32 = 9;

pub fn name(pid: u32) -> String {
    format!("Local\\RustDesktopIcons.Hide.{pid}")
}

pub fn key(path: &str) -> String {
    path.to_lowercase()
}

pub fn encode<S: AsRef<str>>(paths: &[S]) -> Vec<u16> {
    let text = paths.iter().map(|p| key(p.as_ref())).collect::<Vec<_>>().join("\n");
    let mut out: Vec<u16> = text.encode_utf16().collect();
    out.truncate(CHARS);
    out
}

pub fn decode(data: &[u16]) -> HashSet<String> {
    String::from_utf16_lossy(data).split('\n').filter(|s| !s.is_empty()).map(String::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_lowercases_and_skips_empty() {
        let set = decode(&encode(&["C:\\Users\\A\\Desktop\\Été.txt", "c:\\x\\b.lnk"]));
        assert_eq!(set.len(), 2);
        assert!(set.contains("c:\\users\\a\\desktop\\été.txt") && set.contains("c:\\x\\b.lnk"));
        assert!(decode(&encode::<&str>(&[])).is_empty());
    }

    #[test]
    fn encode_is_capped() {
        let long = "a".repeat(CHARS + 10);
        assert_eq!(encode(&[long]).len(), CHARS);
    }
}
