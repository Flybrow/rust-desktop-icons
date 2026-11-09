use crate::domain::{Config, Tab};
use std::{
    env,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn root() -> PathBuf {
    env::var_os("LOCALAPPDATA").map_or_else(env::temp_dir, PathBuf::from).join("RustDesktopIcons")
}

pub fn fence_dir(id: u64) -> PathBuf {
    root().join("fences").join(id.to_string())
}

pub fn tab_dir(t: &Tab) -> PathBuf {
    t.portal.clone().unwrap_or_else(|| fence_dir(t.id))
}

pub fn load() -> (Config, Option<PathBuf>) {
    load_from(&root())
}

pub fn save(c: &Config) -> io::Result<()> {
    save_to(&root(), c)
}

fn parse(p: &Path) -> Option<Config> {
    Config::parse(&fs::read(p).ok()?).ok()
}

fn load_from(dir: &Path) -> (Config, Option<PathBuf>) {
    let (main, bak) = (dir.join("config.json"), dir.join("config.json.bak"));
    if let Some(c) = parse(&main) {
        return (c.sanitized(), None);
    }
    let bad = main.exists().then(|| {
        let secs = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let bad = dir.join(format!("config.json.bad-{secs}"));
        fs::rename(&main, &bad).map(|_| bad)
    });
    (parse(&bak).unwrap_or_default().sanitized(), bad.and_then(Result::ok))
}

fn save_to(dir: &Path, c: &Config) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let (tmp, main) = (dir.join("config.json.tmp"), dir.join("config.json"));
    let mut f = fs::File::create(&tmp)?;
    io::Write::write_all(&mut f, &serde_json::to_vec_pretty(c)?)?;
    f.sync_all()?;
    drop(f);
    if parse(&main).is_some() {
        fs::copy(&main, dir.join("config.json.bak"))?;
    }
    fs::rename(tmp, main)
}

pub fn unique(dir: &Path, name: &OsStr) -> PathBuf {
    unique_by(dir, name, |p| p.exists())
}

pub fn unique_by(dir: &Path, name: &OsStr, taken: impl Fn(&Path) -> bool) -> PathBuf {
    let first = dir.join(name);
    if !taken(&first) {
        return first;
    }
    let p = Path::new(name);
    let stem = p.file_stem().unwrap_or(name).to_string_lossy();
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    (2..).map(|n| dir.join(format!("{stem} ({n}){ext}"))).find(|p| !taken(p)).unwrap_or(first)
}

fn copy_all(src: &Path, dst: &Path) -> io::Result<()> {
    if !src.is_dir() {
        return fs::copy(src, dst).map(|_| ());
    }
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        copy_all(&e.path(), &dst.join(e.file_name()))?;
    }
    Ok(())
}

fn remove_all(p: &Path) -> io::Result<()> {
    if p.is_dir() { fs::remove_dir_all(p) } else { fs::remove_file(p) }
}

fn relocate(src: &Path, dst: &Path) -> io::Result<()> {
    if let Err(e) = copy_all(src, dst) {
        let _ = remove_all(dst);
        return Err(e);
    }
    remove_all(src)
}

fn inside(dir: &Path, src: &Path) -> bool {
    let canon = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    canon(dir).starts_with(canon(src))
}

pub fn move_into(src: &Path, dir: &Path) -> io::Result<PathBuf> {
    let name = src.file_name().ok_or(io::ErrorKind::InvalidInput)?;
    if src.is_dir() && inside(dir, src) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    fs::create_dir_all(dir)?;
    let dst = unique(dir, name);
    fs::rename(src, &dst).or_else(|e| if e.kind() == io::ErrorKind::CrossesDevices { relocate(src, &dst) } else { Err(e) })?;
    crate::shell::notify_moved(src, &dst);
    crate::undo::moved(src, &dst);
    Ok(dst)
}

pub fn evacuate(dir: &Path, to: &Path) -> (Vec<PathBuf>, Vec<String>) {
    let (mut moved, mut errors) = (vec![], vec![]);
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        match move_into(&e.path(), to) {
            Ok(p) => moved.push(p),
            Err(err) => errors.push(format!("{} ({err})", name(&e.path()))),
        }
    }
    if errors.is_empty() {
        let _ = fs::remove_dir(dir);
    }
    (moved, errors)
}

pub fn in_fences(p: &Path) -> bool {
    p.parent().and_then(Path::parent).is_some_and(|q| crate::shell::same_path(q, &root().join("fences")))
}

pub fn ensure(dirs: &[PathBuf]) -> Vec<String> {
    dirs.iter().filter_map(|d| fs::create_dir_all(d).err().map(|e| format!("{} ({e})", d.display()))).collect()
}

pub fn tab_dirs() -> Vec<PathBuf> {
    fs::read_dir(root().join("fences")).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()
}

pub fn filled_dirs() -> Vec<u64> {
    tab_dirs()
        .iter()
        .filter(|d| fs::read_dir(d).is_ok_and(|mut r| r.next().is_some()))
        .filter_map(|d| d.file_name()?.to_str()?.parse().ok())
        .collect()
}

pub fn name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

#[cfg(test)]
mod tests;
