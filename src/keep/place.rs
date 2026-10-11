use super::{pin_of, soon};
use crate::{app::change, domain::pins::Pin, rules, shell, store};
use std::path::PathBuf;
use windows::Win32::Foundation::HWND;

fn commit(fence: u64, tab: u64, paths: Vec<PathBuf>) {
    paths.iter().cloned().for_each(rules::ignore);
    let pins: Vec<Pin> = paths.into_iter().map(pin_of).collect();
    change(|c| c.pin(fence, tab, pins));
    soon();
}

pub fn place(paths: &[PathBuf], fence: u64, tab: u64) -> Vec<String> {
    let Some(desk) = shell::desktop() else { return vec!["Desktop".into()] };
    let (mut done, mut errors) = (vec![], vec![]);
    for p in paths {
        let result = if shell::on_desktop(p).is_some() {
            Ok(p.clone())
        } else if store::in_fences(p) {
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
    let (local, other): (Vec<PathBuf>, Vec<PathBuf>) = files.iter().cloned().partition(|p| cut && shell::on_desktop(p).is_some());
    if !local.is_empty() {
        commit(fence, tab, local);
    }
    if other.is_empty() {
        return true;
    }
    let mut dests: Vec<PathBuf> = vec![];
    for p in &other {
        let name = p.file_name().unwrap_or_default();
        let dst = store::unique_by(&desk, name, |d| d.exists() || dests.iter().any(|x| x == d));
        dests.push(dst);
    }
    if !shell::transfer_to(h, &other, &dests, cut) {
        return false;
    }
    commit(fence, tab, dests.into_iter().filter(|d| d.exists()).collect());
    true
}
