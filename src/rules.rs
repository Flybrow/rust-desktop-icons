use crate::{
    app::with,
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

pub fn watch(tray: HWND) {
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

fn uncover(paths: &[PathBuf]) -> bool {
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
            let target = cfg.rule_for(&kinds);
            if let (Some((f, t)), true) = (target, keep) {
                errors.extend(keep::place(std::slice::from_ref(&p), f, t));
                moved += 1;
            } else if let Some(tab) = target.and_then(|(f, t)| cfg.tab(f, t)) {
                match store::move_into(&p, &store::tab_dir(tab)) {
                    Ok(_) => moved += 1,
                    Err(e) => errors.push(format!("{} ({e})", store::name(&p))),
                }
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
