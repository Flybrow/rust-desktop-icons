use super::{pin_of, soon};
use crate::{app::change, domain::pins::Pin, rules, shell, store};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
use windows::Win32::Foundation::HWND;

fn on_desktop(p: &Path) -> bool {
    shell::desktops().iter().any(|d| p.parent().is_some_and(|q| shell::same_path(q, d)))
}

fn commit(fence: u64, tab: u64, paths: Vec<PathBuf>) {
    paths.iter().cloned().for_each(rules::ignore);
    let pins: Vec<Pin> = paths.into_iter().map(pin_of).collect();
    change(|c| c.pin(fence, tab, pins));
    soon();
}

pub fn place(paths: &[PathBuf], fence: u64, tab: u64) -> Vec<String> {
    let Some(desk) = shell::desktop() else { return vec!["Desktop".into()] };
    let fences = store::root().join("fences");
    let (mut done, mut errors) = (vec![], vec![]);
    for p in paths {
        let result = if on_desktop(p) {
            Ok(p.clone())
        } else if p.parent().and_then(Path::parent).is_some_and(|q| shell::same_path(q, &fences)) {
            store::move_into(p, &desk).map_err(|e| e.to_string())
        } else {
            shell::link_into(p, &desk).map_err(|e| e.to_string())
        };
        match result {
            Ok(dst) => done.push(dst),
            Err(e) => errors.push(format!("{} ({e})", store::name(p))),
        }
    }
    commit(fence, tab, done);
    errors
}

pub fn paste(h: HWND, files: &[PathBuf], cut: bool, fence: u64, tab: u64) -> bool {
    let Some(desk) = shell::desktop() else { return false };
    let (local, other): (Vec<PathBuf>, Vec<PathBuf>) = files.iter().cloned().partition(|p| cut && on_desktop(p));
    if !local.is_empty() {
        commit(fence, tab, local);
    }
    if other.is_empty() {
        return true;
    }
    let list = || fs::read_dir(&desk).into_iter().flatten().flatten().map(|e| e.path()).collect::<HashSet<_>>();
    let before = list();
    if !shell::transfer(h, &other, &desk, cut) {
        return false;
    }
    commit(fence, tab, list().difference(&before).cloned().collect());
    true
}
