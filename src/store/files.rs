use crate::win::wide;
use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};
use windows::{
    Win32::Storage::FileSystem::{COPY_FILE_FAIL_IF_EXISTS, CopyFileExW, MOVE_FILE_FLAGS, MoveFileExW},
    core::PCWSTR,
};

const TRIES: usize = 100;

pub fn unique(dir: &Path, name: &OsStr) -> PathBuf {
    unique_by(dir, name, |p| p.exists())
}

pub fn candidates<'a>(dir: &'a Path, name: &'a OsStr) -> impl Iterator<Item = PathBuf> + 'a {
    let p = Path::new(name);
    let stem = p.file_stem().unwrap_or(name).to_string_lossy().into_owned();
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    std::iter::once(dir.join(name)).chain((2..).map(move |n| dir.join(format!("{stem} ({n}){ext}"))))
}

pub fn unique_by(dir: &Path, name: &OsStr, taken: impl Fn(&Path) -> bool) -> PathBuf {
    candidates(dir, name).find(|p| !taken(p)).unwrap_or_else(|| dir.join(name))
}

pub fn put_new(dir: &Path, name: &OsStr, mut put: impl FnMut(&Path) -> io::Result<()>) -> io::Result<PathBuf> {
    let mut lost: Vec<PathBuf> = vec![];
    loop {
        let dst = unique_by(dir, name, |p| p.symlink_metadata().is_ok() || lost.iter().any(|l| l == p));
        match put(&dst) {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists && lost.len() < TRIES => lost.push(dst),
            r => return r.map(|_| dst),
        }
    }
}

fn long(p: &Path) -> io::Result<Vec<u16>> {
    let abs = std::path::absolute(p)?.display().to_string();
    let verbatim = if abs.starts_with(r"\\?\") {
        abs
    } else if let Some(unc) = abs.strip_prefix(r"\\") {
        format!(r"\\?\UNC\{unc}")
    } else {
        format!(r"\\?\{abs}")
    };
    Ok(wide(&verbatim))
}

fn os(e: windows::core::Error) -> io::Error {
    let code = e.code().0 as u32;
    if code >> 16 == 0x8007 { io::Error::from_raw_os_error((code & 0xFFFF) as i32) } else { io::Error::other(e) }
}

pub fn rename_new(src: &Path, dst: &Path) -> io::Result<()> {
    let (s, d) = (long(src)?, long(dst)?);
    unsafe { MoveFileExW(PCWSTR(s.as_ptr()), PCWSTR(d.as_ptr()), MOVE_FILE_FLAGS(0)) }.map_err(os)
}

pub fn copy_new(src: &Path, dst: &Path) -> io::Result<()> {
    let (s, d) = (long(src)?, long(dst)?);
    unsafe { CopyFileExW(PCWSTR(s.as_ptr()), PCWSTR(d.as_ptr()), None, None, None, COPY_FILE_FAIL_IF_EXISTS) }.map_err(os)
}

fn delete(p: &Path) -> io::Result<()> {
    fs::remove_file(p).or_else(|e| {
        let mut perm = fs::symlink_metadata(p)?.permissions();
        if !perm.readonly() {
            return Err(e);
        }
        #[allow(clippy::permissions_set_readonly_false)]
        perm.set_readonly(false);
        fs::set_permissions(p, perm)?;
        fs::remove_file(p)
    })
}

#[derive(Default)]
struct Copied {
    made: Vec<PathBuf>,
    files: Vec<(PathBuf, PathBuf, u64, Option<SystemTime>)>,
    dirs: Vec<PathBuf>,
    kept: Vec<PathBuf>,
}

fn copy_all(src: &Path, dst: &Path, c: &mut Copied) -> io::Result<()> {
    let m = fs::symlink_metadata(src)?;
    if m.file_type().is_symlink() {
        c.kept.push(src.to_path_buf());
        return Ok(());
    }
    if !m.is_dir() {
        copy_new(src, dst)?;
        c.made.push(dst.to_path_buf());
        c.files.push((src.to_path_buf(), dst.to_path_buf(), m.len(), m.modified().ok()));
        return Ok(());
    }
    fs::create_dir(dst)?;
    c.made.push(dst.to_path_buf());
    c.dirs.push(src.to_path_buf());
    for e in fs::read_dir(src)? {
        let e = e?;
        copy_all(&e.path(), &dst.join(e.file_name()), c)?;
    }
    Ok(())
}

fn unmake(made: &[PathBuf]) {
    for p in made.iter().rev() {
        let _ = if p.is_dir() { fs::remove_dir(p) } else { delete(p) };
    }
}

pub(super) fn relocate(src: &Path, dst: &Path) -> io::Result<()> {
    if fs::symlink_metadata(src)?.file_type().is_symlink() {
        return Err(io::ErrorKind::Unsupported.into());
    }
    let mut c = Copied::default();
    if let Err(e) = copy_all(src, dst, &mut c) {
        unmake(&c.made);
        return Err(e);
    }
    for (from, to, len, time) in c.files {
        let same = fs::symlink_metadata(&from).is_ok_and(|m| m.len() == len && m.modified().ok() == time);
        if !same || delete(&from).is_err() {
            let _ = delete(&to);
            c.kept.push(from);
        }
    }
    for d in c.dirs.iter().rev() {
        let _ = fs::remove_dir(d);
    }
    if c.kept.is_empty() {
        return Ok(());
    }
    for p in c.made.iter().rev().filter(|p| p.is_dir()) {
        let _ = fs::remove_dir(p);
    }
    let names: Vec<String> = c.kept.iter().map(|p| super::name(p)).collect();
    Err(io::Error::other(format!("kept in place, changed or linked: {}", names.join(", "))))
}

pub(super) fn inside(dir: &Path, src: &Path) -> bool {
    let canon = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    canon(dir).starts_with(canon(src))
}

pub fn move_into(src: &Path, dir: &Path) -> io::Result<PathBuf> {
    let name = src.file_name().ok_or(io::ErrorKind::InvalidInput)?;
    if src.is_dir() && inside(dir, src) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    fs::create_dir_all(dir)?;
    let dst = put_new(dir, name, |dst| {
        rename_new(src, dst).or_else(|e| if e.kind() == io::ErrorKind::CrossesDevices { relocate(src, dst) } else { Err(e) })
    })?;
    crate::shell::notify_moved(src, &dst);
    crate::undo::moved(src, &dst);
    Ok(dst)
}

pub fn evacuate(dir: &Path, to: &Path) -> (Vec<PathBuf>, Vec<String>) {
    let (mut moved, mut errors) = (vec![], vec![]);
    let entries = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return (moved, errors),
        Err(e) => return (moved, vec![format!("{} ({e})", dir.display())]),
    };
    for e in entries {
        match e.map(|e| e.path()) {
            Ok(p) => match move_into(&p, to) {
                Ok(p) => moved.push(p),
                Err(err) => errors.push(format!("{} ({err})", super::name(&p))),
            },
            Err(err) => errors.push(format!("{} ({err})", dir.display())),
        }
    }
    if errors.is_empty() {
        let _ = fs::remove_dir(dir);
    }
    (moved, errors)
}
