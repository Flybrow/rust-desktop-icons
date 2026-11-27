use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use windows::{
    Win32::Storage::FileSystem::{MOVEFILE_DELAY_UNTIL_REBOOT, MoveFileExW},
    core::HSTRING,
};

fn intact(p: &Path, digest: &[u8]) -> bool {
    fs::read(p).is_ok_and(|b| Sha256::digest(&b)[..] == *digest)
}

fn write(tmp: &Path, path: &Path, bytes: &[u8]) -> Option<()> {
    let mut f = fs::File::create(tmp).ok()?;
    f.write_all(bytes).ok()?;
    f.sync_all().ok()?;
    drop(f);
    fs::rename(tmp, path).ok()
}

fn sweep(dir: &Path, keep: &Path) {
    for old in fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p != keep) {
        if fs::remove_file(&old).is_err() {
            let _ = unsafe { MoveFileExW(&HSTRING::from(old.as_os_str()), None, MOVEFILE_DELAY_UNTIL_REBOOT) };
        }
    }
}

pub fn ensure(dir: &Path, bytes: &[u8]) -> Option<PathBuf> {
    let digest = Sha256::digest(bytes);
    let hash: String = digest.iter().take(6).map(|b| format!("{b:02x}")).collect();
    let path = dir.join(format!("rdi_hook-{hash}.dll"));
    if !intact(&path, &digest) {
        fs::create_dir_all(dir).ok()?;
        write(&dir.join(format!("rdi_hook-{hash}.tmp")), &path, bytes)?;
        if !intact(&path, &digest) {
            return None;
        }
    }
    sweep(dir, &path);
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_damaged_copy_is_rewritten_and_old_ones_go() {
        let dir = std::env::temp_dir().join(format!("rdi-dll-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = ensure(&dir, b"one").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"one");
        fs::write(&path, b"bad").unwrap();
        assert_eq!(ensure(&dir, b"one"), Some(path.clone()));
        assert_eq!(fs::read(&path).unwrap(), b"one", "a copy that does not match is replaced");
        let next = ensure(&dir, b"two").unwrap();
        assert_ne!(next, path);
        let left: Vec<PathBuf> = fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).collect();
        assert_eq!(left, vec![next]);
        fs::remove_dir_all(dir).unwrap();
    }
}
