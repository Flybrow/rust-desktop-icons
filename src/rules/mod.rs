pub mod age;

use crate::{
    app::with,
    domain::Config,
    domain::kind::{candidates, ignored},
    fence,
    i18n::T,
    keep, report, shell,
    shell::hide::Desk,
    store, tray,
    win::buttons_down,
};
use std::{
    collections::HashSet,
    fs,
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
    time::Duration,
};
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::WM_APP};

pub const WM_DESK: u32 = WM_APP + 20;
pub const TIMER: usize = tray::timers::RULES;
const OWN_SHORTCUT: &str = "Rust Desktop Icons.lnk";
const SETTLE: Duration = Duration::from_secs(10);
const RETRY_MS: u32 = 3000;

fn settled(p: &Path) -> bool {
    let Ok(m) = fs::metadata(p) else { return false };
    [m.modified(), m.created()].into_iter().flatten().filter_map(|t| t.elapsed().ok()).all(|age| age >= SETTLE)
}

fn entries(desk: &Path) -> HashSet<PathBuf> {
    fs::read_dir(desk).into_iter().flatten().flatten().map(|e| e.path()).collect()
}

fn movable(p: &Path) -> bool {
    let hidden = fs::metadata(p).is_ok_and(|m| m.file_attributes() & 0x6 != 0);
    !ignored(&store::name(p), hidden, OWN_SHORTCUT)
}

fn put(cfg: &Config, p: &Path, (f, t): (u64, u64), keep: bool) -> Result<bool, String> {
    if keep {
        return keep::place(std::slice::from_ref(&p.to_path_buf()), f, t).into_iter().next().map_or(Ok(true), Err);
    }
    let Some(tab) = cfg.tab(f, t) else { return Ok(false) };
    store::move_into(p, &store::tab_dir(tab)).map(|_| true).map_err(|e| format!("{} ({e})", store::name(p)))
}

pub fn watch(tray: HWND) {
    age::start();
    let Some(desk) = shell::desktop() else { return };
    let (seen, w) = (entries(&desk), shell::Watch::new(tray, &desk, WM_DESK));
    with(|a| (a.seen, a.desk) = (seen, w));
}

pub fn changed() {
    tray::timers::set(TIMER, Some(2000));
}

pub fn ignore(p: PathBuf) {
    with(|a| a.seen.insert(p));
}

pub fn uncover(paths: &[PathBuf]) -> bool {
    if paths.is_empty() || !keep::live() {
        return true;
    }
    if buttons_down() {
        return false;
    }
    if let Some(d) = Desk::open() {
        d.uncover(paths, &fence::footprints());
    }
    true
}

pub fn run(tray: bool, all: bool) {
    crate::undo::step(|| sort(tray, all));
}

fn sort(tray: bool, all: bool) {
    if tray {
        tray::timers::set(TIMER, None);
    }
    keep::reconcile();
    let Some(desk) = shell::desktop() else { return };
    let Some((cfg, seen)) = with(|a| (a.cfg.clone(), a.seen.clone())) else { return };
    let (mut moved, mut errors, mut pending) = (0, vec![], HashSet::new());
    let loose: Vec<PathBuf> = entries(&desk).into_iter().filter(|p| movable(p) && !cfg.is_pinned(p)).collect();
    let fresh: Vec<PathBuf> = loose.iter().filter(|p| !seen.contains(*p)).cloned().collect();
    if all || cfg.auto_sort {
        let keep = cfg.keep_on_desktop && keep::live();
        for p in if all { loose } else { fresh.clone() } {
            if !all && !settled(&p) {
                pending.insert(p);
                continue;
            }
            let target = shell::link_target(&p);
            let kinds = candidates(p.extension().and_then(|e| e.to_str()), p.is_dir(), target.as_deref());
            match cfg.target_for(&store::name(&p), &kinds).map(|ft| put(&cfg, &p, ft, keep)) {
                Some(Ok(true)) => moved += 1,
                Some(Err(e)) => errors.push(e),
                _ => {}
            }
        }
    }
    let left: Vec<PathBuf> = fresh.into_iter().filter(|p| p.exists() && with(|a| a.cfg.is_pinned(p)) == Some(false)).collect();
    if !uncover(&left) {
        pending.extend(left);
    }
    let now = entries(&desk).into_iter().filter(|p| !pending.contains(p)).collect();
    with(|a| a.seen = now);
    if tray && !pending.is_empty() {
        tray::timers::set(TIMER, Some(RETRY_MS));
    }
    if moved > 0 {
        fence::reload_all();
    }
    report::failures(T::ErrMove, &errors);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{File, FileTimes},
        os::windows::fs::{FileTimesExt, OpenOptionsExt},
        time::SystemTime,
    };

    #[test]
    fn files_wait_until_written_and_hidden_ones_stay() {
        let d = std::env::temp_dir().join(format!("rdi-rules-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        let (new, old, copied) = (d.join("new.txt"), d.join("old.txt"), d.join("copied.txt"));
        for p in [&new, &old, &copied] {
            fs::write(p, "x").unwrap();
        }
        let past = SystemTime::now() - Duration::from_secs(60);
        let open = |p: &Path| File::options().write(true).custom_flags(0x0200_0000).open(p).unwrap();
        open(&old).set_times(FileTimes::new().set_modified(past).set_created(past)).unwrap();
        open(&copied).set_times(FileTimes::new().set_modified(past)).unwrap();
        assert!(!settled(&new) && settled(&old) && !settled(&copied) && !settled(&d.join("missing")));
        let hidden = d.join("hidden.txt");
        fs::write(&hidden, "h").unwrap();
        std::process::Command::new("attrib").arg("+h").arg(&hidden).status().unwrap();
        assert!(movable(&old) && !movable(&hidden) && !movable(&d.join("a.crdownload")) && !movable(&d.join(OWN_SHORTCUT)));
        fs::remove_dir_all(d).unwrap();
    }
}
