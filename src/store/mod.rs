mod files;

use crate::domain::{Config, Tab};
pub use files::{candidates, copy_new, evacuate, move_into, put_new, rename_new, unique, unique_by};
#[cfg(test)]
use files::{inside, relocate};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, PartialEq)]
pub enum Damage {
    SetAside(PathBuf),
    Held(PathBuf),
}

static HELD: Mutex<Option<PathBuf>> = Mutex::new(None);
static GOOD: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

pub fn root() -> PathBuf {
    env::var_os("LOCALAPPDATA").map_or_else(env::temp_dir, PathBuf::from).join("RustDesktopIcons")
}

pub fn fence_dir(id: u64) -> PathBuf {
    root().join("fences").join(id.to_string())
}

pub fn tab_dir(t: &Tab) -> PathBuf {
    t.portal.clone().unwrap_or_else(|| fence_dir(t.id))
}

pub fn load() -> (Config, Option<Damage>) {
    let (mut c, damage) = load_from(&root());
    if cfg!(debug_assertions) && std::env::var_os("RDI_GLASS").is_some() {
        c.look.glass = true;
        c.fences.iter_mut().for_each(|f| f.look.glass = true);
    }
    (c, damage)
}

pub fn save(c: &Config) -> io::Result<()> {
    save_to(&root(), c)
}

fn parse(p: &Path) -> Option<Config> {
    Config::parse(&fs::read(p).ok()?).ok()
}

fn read(p: &Path) -> io::Result<Vec<u8>> {
    let mut last = fs::read(p);
    for _ in 0..10 {
        match &last {
            Err(e) if e.kind() != io::ErrorKind::NotFound => thread::sleep(Duration::from_millis(200)),
            _ => break,
        }
        last = fs::read(p);
    }
    last
}

fn good(main: &Path, ok: Option<bool>) -> bool {
    let mut g = GOOD.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ok) = ok {
        g.retain(|p| p != main);
        g.extend(ok.then(|| main.to_path_buf()));
    }
    g.iter().any(|p| p == main)
}

fn hold(main: PathBuf) -> Option<Damage> {
    *HELD.lock().unwrap_or_else(|e| e.into_inner()) = Some(main.clone());
    Some(Damage::Held(main))
}

fn load_from(dir: &Path) -> (Config, Option<Damage>) {
    let (main, bak) = (dir.join("config.json"), dir.join("config.json.bak"));
    let fallback = || parse(&bak).unwrap_or_default().sanitized();
    good(&main, Some(false));
    let bytes = match read(&main) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return (fallback(), None),
        Err(_) => return (fallback(), hold(main)),
        Ok(b) => b,
    };
    if let Ok(c) = Config::parse(&bytes) {
        good(&main, Some(true));
        return (c.sanitized(), None);
    }
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let bad = dir.join(format!("config.json.bad-{secs}"));
    match fs::rename(&main, &bad) {
        Ok(()) => (fallback(), Some(Damage::SetAside(bad))),
        Err(_) => (fallback(), hold(main)),
    }
}

fn save_to(dir: &Path, c: &Config) -> io::Result<()> {
    if HELD.lock().unwrap_or_else(|e| e.into_inner()).as_deref() == Some(&dir.join("config.json")) {
        return Err(io::Error::other("config.json is kept as it was: restart to save again"));
    }
    fs::create_dir_all(dir)?;
    let (tmp, main) = (dir.join("config.json.tmp"), dir.join("config.json"));
    let mut f = fs::File::create(&tmp)?;
    io::Write::write_all(&mut f, &serde_json::to_vec_pretty(c)?)?;
    f.sync_all()?;
    drop(f);
    if good(&main, None) {
        fs::copy(&main, dir.join("config.json.bak"))?;
    }
    fs::rename(tmp, &main)?;
    good(&main, Some(true));
    Ok(())
}

pub fn in_fences(p: &Path) -> bool {
    p.parent().and_then(Path::parent).is_some_and(|q| crate::shell::same_path(q, &root().join("fences")))
}

pub fn ensure(dirs: &[PathBuf]) -> Vec<String> {
    dirs.iter().filter_map(|d| fs::create_dir_all(d).err().map(|e| format!("{} ({e})", d.display()))).collect()
}

pub fn tab_dirs() -> io::Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(root().join("fences")) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
        r => r?,
    };
    let paths = entries.map(|e| e.map(|e| e.path())).collect::<io::Result<Vec<_>>>()?;
    Ok(paths.into_iter().filter(|p| p.is_dir()).collect())
}

fn ids(filled: bool) -> Vec<u64> {
    tab_dirs()
        .unwrap_or_default()
        .iter()
        .filter(|d| !filled || fs::read_dir(d).is_ok_and(|mut r| r.next().is_some()))
        .filter_map(|d| d.file_name()?.to_str()?.parse().ok())
        .collect()
}

pub fn dir_ids() -> Vec<u64> {
    ids(false)
}

pub fn filled_dirs() -> Vec<u64> {
    ids(true)
}

pub fn name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn ours(name: &str) -> bool {
    name.starts_with("config.json") || name.ends_with(".log") || name.ends_with(".log.old") || name.starts_with("rdi_hook-")
}

fn clear(dir: &Path) {
    for p in fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()) {
        if p.is_file() && ours(&name(&p)) {
            let _ = fs::remove_file(&p);
        }
    }
    let _ = fs::remove_dir(dir);
}

pub fn remove_app_files() {
    let r = root();
    let _ = fs::remove_dir(r.join("fences"));
    clear(&r.join("hook"));
    clear(&r);
}

#[cfg(test)]
mod tests;
