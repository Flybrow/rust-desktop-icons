use super::{apply, live, pin_of, restore};
use crate::{
    app::{change, rebuild, with},
    domain::pins::Pin,
    fence,
    i18n::T,
    report, rules, shell, store, tray,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    thread,
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{PostMessageW, WM_APP},
};

pub const WM_KEEP: u32 = WM_APP + 21;
static BUSY: AtomicBool = AtomicBool::new(false);

struct Job {
    keep: bool,
    desk: PathBuf,
    tabs: Vec<(u64, u64, PathBuf, Vec<Pin>)>,
}

#[derive(Default)]
struct Done {
    keep: bool,
    rebuild: bool,
    pinned: Vec<(u64, u64, Vec<Pin>)>,
    back: Vec<PathBuf>,
    public: Vec<(PathBuf, PathBuf)>,
    errors: Vec<String>,
}

fn run(job: Job, rebuild: bool) -> Done {
    let mut d = Done { keep: job.keep, rebuild, ..Done::default() };
    for (fence, tab, dir, pins) in job.tabs {
        if job.keep {
            if fs::read_dir(&dir).is_ok_and(|mut e| e.next().is_some()) {
                let (moved, errors) = store::evacuate(&dir, &job.desk);
                d.errors.extend(errors);
                d.pinned.push((fence, tab, moved.into_iter().map(pin_of).collect()));
            }
            continue;
        }
        for p in pins {
            match store::move_into(&p.path, &dir) {
                Ok(_) => d.back.push(p.path),
                Err(_) if !p.path.exists() => d.back.push(p.path),
                Err(_) if shell::on_desktop(&p.path) == Some(1) => d.public.push((p.path, dir.clone())),
                Err(e) => d.errors.push(format!("{} ({e})", store::name(&p.path))),
            }
        }
    }
    d
}

pub fn sync(rebuild_after: bool) {
    if !live() {
        if rebuild_after {
            rebuild();
        }
        return;
    }
    let (Some(desk), Some(t)) = (shell::desktop(), tray::hwnd()) else { return };
    if BUSY.swap(true, Ordering::AcqRel) {
        return;
    }
    let Some(job) = with(|a| Job {
        keep: a.cfg.keep_on_desktop,
        desk,
        tabs: a
            .cfg
            .fences
            .iter()
            .flat_map(|f| f.tabs.iter().filter(|t| t.portal.is_none()).map(|t| (f.id, t.id, store::tab_dir(t), t.pins.clone())))
            .collect(),
    }) else {
        return BUSY.store(false, Ordering::Release);
    };
    let tray = t.0 as isize;
    thread::spawn(move || {
        let raw = Box::into_raw(Box::new(run(job, rebuild_after)));
        if unsafe { PostMessageW(Some(HWND(tray as _)), WM_KEEP, WPARAM(0), LPARAM(raw as isize)) }.is_err() {
            drop(unsafe { Box::from_raw(raw) });
            BUSY.store(false, Ordering::Release);
        }
    });
}

pub fn finish(lp: LPARAM) {
    let d = *unsafe { Box::from_raw(lp.0 as *mut Done) };
    BUSY.store(false, Ordering::Release);
    let (mut back, mut errors) = (d.back, d.errors);
    for (p, dir) in d.public {
        let dst = store::unique(&dir, p.file_name().unwrap_or_default());
        if tray::hwnd().is_some_and(|t| shell::transfer_to(t, std::slice::from_ref(&p), std::slice::from_ref(&dst), true)) {
            back.push(p);
        } else {
            errors.push(store::name(&p));
        }
    }
    d.pinned.iter().flat_map(|(_, _, pins)| pins).for_each(|p| rules::ignore(p.path.clone()));
    change(|c| {
        for (f, t, pins) in d.pinned {
            c.pin(f, t, pins);
        }
        c.unpin(&back);
    });
    report::failures(T::ErrMove, &errors);
    if with(|a| a.cfg.keep_on_desktop) != Some(d.keep) {
        return sync(true);
    }
    if d.keep {
        apply()
    } else {
        restore()
    }
    if d.rebuild { rebuild() } else { fence::reload_all() }
}
