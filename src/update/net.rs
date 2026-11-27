use super::{Res, release::trusted};
use std::{io::Read, time::Duration};

pub fn stream(url: &str, limit: u64, mut progress: impl FnMut(u64, u64)) -> Res<Vec<u8>> {
    if !trusted(url) {
        return Err("untrusted url".into());
    }
    let secs = |s| Some(Duration::from_secs(s));
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .timeout_resolve(secs(20))
        .timeout_connect(secs(20))
        .timeout_recv_response(secs(30))
        .timeout_per_read(secs(30))
        .build()
        .into();
    let mut r = agent
        .get(url)
        .header("User-Agent", concat!("rust-desktop-icons/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .call()?;
    let total = r.headers().get("content-length").and_then(|v| v.to_str().ok()?.parse().ok()).unwrap_or(0);
    let mut reader = r.body_mut().with_config().limit(limit).reader();
    let (mut out, mut buf) = (Vec::with_capacity((total as usize).min(limit as usize)), vec![0u8; 64 << 10]);
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
    fn other_hosts_are_refused_before_any_connection() {
        let err = get("https://evil.invalid/rust-desktop-icons.msi", 1 << 20).unwrap_err();
        assert_eq!(err.to_string(), "untrusted url");
    }
}
