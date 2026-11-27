use super::{
    REPO,
    notes::{Notes, parse as notes},
};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: [u64; 3],
    pub notes: Notes,
    pub msi: String,
    pub sum: String,
    pub sources: Vec<(String, String)>,
    pub translations: Vec<(String, Notes)>,
}

impl Release {
    pub fn label(&self) -> String {
        label(self.version)
    }

    pub fn notes_in(&self, lang: &str) -> &Notes {
        self.translations.iter().find(|(l, n)| l == lang && !n.blocks.is_empty()).map_or(&self.notes, |(_, n)| n)
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

const NOTES: &str = "release-notes.";

pub fn from_json(rel: &Value) -> Option<Release> {
    let asset = |suffix: &str| {
        rel["assets"].as_array()?.iter().find(|a| a["name"].as_str().is_some_and(|n| n.ends_with(suffix)))?["browser_download_url"]
            .as_str()
            .map(String::from)
    };
    let sources = rel["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| {
            let lang = a["name"].as_str()?.strip_prefix(NOTES)?.strip_suffix(".md")?;
            Some((lang.to_string(), a["browser_download_url"].as_str()?.to_string()))
        })
        .collect();
    Some(Release {
        version: parse(rel["tag_name"].as_str()?)?,
        notes: notes(rel["body"].as_str().unwrap_or_default()),
        msi: asset(".msi")?,
        sum: asset(".msi.sha256")?,
        sources,
        translations: vec![],
    })
}

pub fn trusted(url: &str) -> bool {
    let Some((host, path)) = url.strip_prefix("https://").and_then(|r| r.split_once('/')) else { return false };
    let plain = path.chars().all(|c| c.is_ascii_alphanumeric() || "-._/~+".contains(c));
    let segments = path.split('/').all(|s| !s.is_empty() && s != "." && s != "..");
    let under = match host {
        "github.com" => path.starts_with(&format!("{REPO}/releases/download/")),
        "api.github.com" => path.starts_with(&format!("repos/{REPO}/releases/")),
        _ => false,
    };
    plain && segments && under
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
                {"name": "rust-desktop-icons-0.9.4-x64.msi", "browser_download_url": format!("{base}a.msi")},
                {"name": "release-notes.fr.md", "browser_download_url": format!("{base}release-notes.fr.md")}
            ]
        });
        let mut r = from_json(&rel).unwrap();
        assert_eq!(r.sources, vec![("fr".to_string(), format!("{base}release-notes.fr.md"))]);
        assert_eq!(r.notes_in("fr").title, "What's new");
        r.translations.push((
            "fr".into(),
            notes(
                "## Nouveautés
- **Rapide**",
            ),
        ));
        assert_eq!((r.notes_in("fr").title.as_str(), r.notes_in("de").title.as_str()), ("Nouveautés", "What's new"));
        assert_eq!((r.label(), r.notes.title.as_str(), r.notes.blocks.len()), ("0.9.4".to_string(), "What's new", 1));
        assert_eq!((r.msi, r.sum), (format!("{base}a.msi"), format!("{base}a.msi.sha256")));
        assert!(from_json(&rel).unwrap().newer_than("0.9.3") && !from_json(&rel).unwrap().newer_than("0.9.4"));
        assert_eq!(from_json(&json!({"tag_name": "v1.0.0", "assets": []})), None);
        assert_eq!(from_json(&json!({"assets": []})), None);
    }

    #[test]
    fn only_this_repository_releases() {
        let ok = "https://github.com/Flybrow/rust-desktop-icons/releases/download/v1.0.0/rust-desktop-icons-1.0.0-x64.msi";
        assert!(trusted(ok) && trusted(&format!("{ok}.sha256")));
        assert!(trusted("https://api.github.com/repos/Flybrow/rust-desktop-icons/releases/latest"));
        for bad in [
            "https://github.com/someone/else/releases/download/v1.0.0/a.msi",
            "http://github.com/Flybrow/rust-desktop-icons/releases/download/v1.0.0/a.msi",
            "https://github.com/Flybrow/rust-desktop-icons/releases/download/../../../evil/x.msi",
            "https://github.com/Flybrow/rust-desktop-icons/releases/download/v1/%2e%2e/a.msi",
            "https://github.com/Flybrow/rust-desktop-icons/releases/download/./a.msi",
            "https://github.com/Flybrow/rust-desktop-icons/releases/download//a.msi",
            "https://github.com/Flybrow/rust-desktop-icons/releases/download/v1/a.msi?x=1",
            "https://github.com.evil.io/Flybrow/rust-desktop-icons/releases/download/v1/a.msi",
            "https://github.com@evil.io/Flybrow/rust-desktop-icons/releases/download/v1/a.msi",
            "https://github.com:8443/Flybrow/rust-desktop-icons/releases/download/v1/a.msi",
            "https://api.github.com/repos/Flybrow/rust-desktop-icons/issues",
            "https://api.github.com/",
            "https://github.com",
            "",
        ] {
            assert!(!trusted(bad), "{bad}");
        }
        assert!(!trusted(&ok.replace('/', "\\")));
    }

    #[test]
    fn progress() {
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
