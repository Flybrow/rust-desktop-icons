mod install;
mod net;
mod release;

pub use install::install;
pub use release::{Release, permille, plain};
use std::{env, error::Error, fs, io::Write, path::PathBuf};

pub type Res<T> = Result<T, Box<dyn Error>>;

const REPO: &str = "Flybrow/rust-desktop-icons";
const MAX_MSI: u64 = 64 << 20;

pub fn current() -> String {
    std::env::var("RDI_FAKE_VERSION").ok().filter(|_| cfg!(debug_assertions)).unwrap_or_else(|| env!("CARGO_PKG_VERSION").into())
}

pub fn installed(code: u32) -> bool {
    matches!(code, 0 | 1641 | 3010)
}

pub fn check() -> Res<Option<Release>> {
    let json = net::get(&format!("https://api.github.com/repos/{REPO}/releases/latest"), 1 << 20)?;
    let rel = release::from_json(&serde_json::from_slice(&json)?).ok_or("unexpected release format")?;
    Ok(rel.newer_than(&current()).then_some(rel))
}

pub fn download(rel: &Release, progress: impl FnMut(u64, u64)) -> Res<PathBuf> {
    let sum = String::from_utf8(net::get(&rel.sum, 1024)?)?;
    let msi = net::stream(&rel.msi, MAX_MSI, progress)?;
    if !release::verify(&msi, &sum) {
        return Err("checksum mismatch".into());
    }
    let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let path = env::temp_dir().join(format!("rust-desktop-icons-{}-{}-{nonce}.msi", rel.label(), std::process::id()));
    fs::OpenOptions::new().write(true).create_new(true).open(&path)?.write_all(&msi)?;
    Ok(path)
}
