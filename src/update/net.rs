use super::{REPO, Res};
use std::{io::Read, time::Duration};

fn trusted(url: &str) -> bool {
    url.starts_with(&format!("https://github.com/{REPO}/releases/download/")) || url.starts_with("https://api.github.com/")
}

pub fn stream(url: &str, limit: u64, mut progress: impl FnMut(u64, u64)) -> Res<Vec<u8>> {
    if !trusted(url) {
        return Err("untrusted url".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(120))).build().into();
    let mut r = agent
        .get(url)
        .header("User-Agent", concat!("rust-desktop-icons/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .call()?;
    let total = r.headers().get("content-length").and_then(|v| v.to_str().ok()?.parse().ok()).unwrap_or(0);
    let mut reader = r.body_mut().with_config().limit(limit).reader();
    let (mut out, mut buf) = (vec![], vec![0u8; 64 << 10]);
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            return Ok(out);
        }
        out.extend_from_slice(&buf[..n]);
        progress(out.len() as u64, total);
    }
}

pub fn get(url: &str, limit: u64) -> Res<Vec<u8>> {
    stream(url, limit, |_, _| {})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_this_repository() {
        assert!(trusted("https://api.github.com/repos/Flybrow/rust-desktop-icons/releases/latest"));
        assert!(trusted("https://github.com/Flybrow/rust-desktop-icons/releases/download/v1.0.0/a.msi"));
        assert!(!trusted("https://github.com/someone/else/releases/download/v1.0.0/a.msi"));
        assert!(!trusted("http://github.com/Flybrow/rust-desktop-icons/releases/download/v1.0.0/a.msi"));
    }
}
