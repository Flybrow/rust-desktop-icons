use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: [u64; 3],
    pub notes: String,
    pub msi: String,
    pub sum: String,
}

impl Release {
    pub fn label(&self) -> String {
        label(self.version)
    }

    pub fn newer_than(&self, current: &str) -> bool {
        parse(current).is_some_and(|c| self.version > c)
    }
}

pub fn label([a, b, c]: [u64; 3]) -> String {
    format!("{a}.{b}.{c}")
}

pub fn parse(v: &str) -> Option<[u64; 3]> {
    let mut it = v.trim().trim_start_matches('v').split('.').map(|p| p.parse().ok());
    let r = [it.next()??, it.next()??, it.next()??];
    it.next().is_none().then_some(r)
}

pub fn from_json(rel: &Value) -> Option<Release> {
    let asset = |suffix: &str| {
        rel["assets"].as_array()?.iter().find(|a| a["name"].as_str().is_some_and(|n| n.ends_with(suffix)))?["browser_download_url"]
            .as_str()
            .map(String::from)
    };
    Some(Release {
        version: parse(rel["tag_name"].as_str()?)?,
        notes: plain(rel["body"].as_str().unwrap_or_default()),
        msi: asset(".msi")?,
        sum: asset(".msi.sha256")?,
    })
}

pub fn plain(md: &str) -> String {
    md.lines()
        .map(|l| l.trim_start_matches('#').trim_start().replace("**", "").replace('`', ""))
        .map(|l| l.strip_prefix("- ").map_or(l.clone(), |r| format!("•  {r}")))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

pub fn verify(data: &[u8], sum: &str) -> bool {
    let hex: String = Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect();
    sum.split_whitespace().next().is_some_and(|s| s.eq_ignore_ascii_case(&hex))
}

pub fn permille(done: u64, total: u64) -> u32 {
    (done.min(total) * 1000).checked_div(total).unwrap_or(0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn versions() {
        assert_eq!(parse("v1.2.3"), Some([1, 2, 3]));
        assert_eq!(parse("0.10.0"), Some([0, 10, 0]));
        assert!(parse("v0.2.0") > parse("0.1.9"));
        assert_eq!(parse("1.2"), None);
        assert_eq!(parse("1.2.3.4"), None);
        assert_eq!(parse("1.x.3"), None);
        assert_eq!(label([0, 9, 3]), "0.9.3");
    }

    #[test]
    fn release_from_github() {
        let base = "https://github.com/Flybrow/rust-desktop-icons/releases/download/v0.9.4/";
        let rel = json!({
            "tag_name": "v0.9.4",
            "body": "## What's new\n- **Fast** `thing`",
            "assets": [
                {"name": "rust-desktop-icons-0.9.4-x64.msi.sha256", "browser_download_url": format!("{base}a.msi.sha256")},
                {"name": "rust-desktop-icons-0.9.4-x64.msi", "browser_download_url": format!("{base}a.msi")}
            ]
        });
        let r = from_json(&rel).unwrap();
        assert_eq!((r.label(), r.notes.as_str()), ("0.9.4".to_string(), "What's new\n•  Fast thing"));
        assert_eq!((r.msi, r.sum), (format!("{base}a.msi"), format!("{base}a.msi.sha256")));
        assert!(from_json(&rel).unwrap().newer_than("0.9.3") && !from_json(&rel).unwrap().newer_than("0.9.4"));
        assert_eq!(from_json(&json!({"tag_name": "v1.0.0", "assets": []})), None);
        assert_eq!(from_json(&json!({"assets": []})), None);
    }

    #[test]
    fn notes_and_progress() {
        assert_eq!(plain("## What's new\n\n### Fixes\n- **Safe** `x`"), "What's new\n\nFixes\n•  Safe x");
        assert_eq!((permille(0, 0), permille(50, 200), permille(300, 200)), (0, 250, 1000));
    }

    #[test]
    fn checksum() {
        let sum = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  app.msi";
        assert!(verify(b"abc", sum));
        assert!(verify(b"abc", &sum.to_uppercase()));
        assert!(!verify(b"abd", sum));
        assert!(!verify(b"abc", ""));
    }
}
