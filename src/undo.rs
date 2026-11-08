use crate::{
    app::with,
    backup,
    domain::{
        Config,
        history::{History, Op},
    },
    i18n::T,
    report, store,
};
use std::{cell::RefCell, fs, io, path::Path};

thread_local! {
    static HISTORY: RefCell<History> = RefCell::new(History::default());
}

fn now() -> Config {
    with(|a| a.cfg.clone()).unwrap_or_default()
}

pub fn step<R>(f: impl FnOnce() -> R) -> R {
    let before = now();
    HISTORY.with(|h| h.borrow_mut().begin(&before));
    let out = f();
    let after = now();
    HISTORY.with(|h| h.borrow_mut().end(&after));
    out
}

fn note(op: Op) {
    let _ = HISTORY.try_with(|h| h.try_borrow_mut().map(|mut h| h.note(op)));
}

pub fn moved(from: &Path, to: &Path) {
    note(Op::Moved(from.to_path_buf(), to.to_path_buf()));
}

pub fn created(p: &Path) {
    note(Op::Created(p.to_path_buf()));
}

pub fn shortcut(vk: u16) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_Z};
    vk == VK_Z.0 && crate::win::key_down(VK_CONTROL)
}

pub fn available() -> bool {
    HISTORY.with(|h| h.borrow().any())
}

fn back(from: &Path, to: &Path) -> io::Result<()> {
    let dir = from.parent().ok_or(io::ErrorKind::InvalidInput)?;
    fs::create_dir_all(dir)?;
    if from.exists() {
        return store::move_into(to, dir).map(|_| ());
    }
    fs::rename(to, from).or_else(|_| store::move_into(to, dir).map(|_| ()))
}

fn revert(ops: &[Op]) -> Vec<String> {
    let mut errors = vec![];
    for op in ops.iter().rev() {
        match op {
            Op::Moved(from, to) if to.exists() => {
                if let Err(e) = back(from, to) {
                    errors.push(format!("{} ({e})", store::name(to)));
                }
            }
            Op::Created(p) if p.exists() => {
                if let Err(e) = fs::remove_file(p) {
                    errors.push(format!("{} ({e})", store::name(p)));
                }
            }
            _ => {}
        }
    }
    errors
}

pub fn undo() {
    let Some(step) = HISTORY.with(|h| h.borrow_mut().pop()) else { return };
    let errors = revert(&step.ops);
    backup::adopt(|c| c.rewind(step.before));
    let after = now();
    HISTORY.with(|h| h.borrow_mut().rebase(&after));
    crate::fence::reload_all();
    report::failures(T::ErrMove, &errors);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::windows::fs::OpenOptionsExt, path::PathBuf};

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rdi-undo-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn moved(from: &Path, dir: &Path) -> Op {
        Op::Moved(from.to_path_buf(), store::move_into(from, dir).unwrap())
    }

    #[test]
    fn files_go_back_in_reverse_order() {
        let d = tmp("order");
        let (desk, a, b) = (d.join("desk"), d.join("a"), d.join("b"));
        fs::create_dir_all(&desk).unwrap();
        fs::write(desk.join("x.txt"), "x").unwrap();
        let first = moved(&desk.join("x.txt"), &a);
        let second = moved(&a.join("x.txt"), &b);
        fs::remove_dir_all(&desk).unwrap();
        fs::write(d.join("link.lnk"), "l").unwrap();
        let errors = revert(&[first, second, Op::Created(d.join("link.lnk"))]);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(fs::read_to_string(desk.join("x.txt")).unwrap(), "x");
        assert!(!a.join("x.txt").exists() && !b.join("x.txt").exists() && !d.join("link.lnk").exists());
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn undo_never_overwrites_or_fails_on_what_changed_since() {
        let d = tmp("changed");
        let (desk, fence) = (d.join("desk"), d.join("fence"));
        fs::create_dir_all(&desk).unwrap();
        for n in ["same.txt", "gone.txt", "busy.txt"] {
            fs::write(desk.join(n), "old").unwrap();
        }
        let ops: Vec<Op> = ["same.txt", "gone.txt", "busy.txt"].iter().map(|n| moved(&desk.join(n), &fence)).collect();
        fs::write(desk.join("same.txt"), "new").unwrap();
        fs::remove_file(fence.join("gone.txt")).unwrap();
        let hold = fs::OpenOptions::new().read(true).share_mode(0).open(fence.join("busy.txt")).unwrap();
        let errors = revert(&ops);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].starts_with("busy.txt"));
        assert_eq!(fs::read_to_string(desk.join("same.txt")).unwrap(), "new");
        assert_eq!(fs::read_to_string(desk.join("same (2).txt")).unwrap(), "old");
        assert!(!desk.join("gone.txt").exists() && fence.join("busy.txt").exists());
        drop(hold);
        assert!(revert(&ops[2..]).is_empty() && desk.join("busy.txt").exists());
        assert!(revert(&[Op::Created(d.join("missing.lnk"))]).is_empty());
        fs::remove_dir_all(d).unwrap();
    }
}
