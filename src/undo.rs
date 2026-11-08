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

pub fn undo() {
    let Some(step) = HISTORY.with(|h| h.borrow_mut().pop()) else { return };
    let mut errors = vec![];
    for op in step.ops.iter().rev() {
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
    backup::adopt(|c| c.rewind(step.before));
    let after = now();
    HISTORY.with(|h| h.borrow_mut().rebase(&after));
    crate::fence::reload_all();
    report::failures(T::ErrMove, &errors);
}
