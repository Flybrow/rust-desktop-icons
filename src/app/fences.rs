use super::{View, with};
use crate::{domain::Fence, fence, i18n::T, report, shell, store, win::*};
use windows::Win32::{Foundation::RECT, UI::WindowsAndMessaging::*};

pub fn recover() {
    let dirs = store::filled_dirs();
    let w = work_area_at((0, 0));
    let at = (w.left + sys_scale(40), w.top + sys_scale(40));
    let found = with(|a| {
        let title = a.t(T::Recovered).to_string();
        a.cfg.recover(&dirs, &title, at).inspect(|_| a.save())
    });
    if let Some(Some(id)) = found {
        report::log(&format!("orphan fence folders recovered into fence {id}"));
        super::check_save();
    }
}

pub fn register_verb() {
    if let Some(label) = with(|a| a.t(T::DesktopVerb)).filter(|_| !cfg!(debug_assertions)) {
        shell::set_desktop_verb(label);
    }
}

pub fn new_fence() {
    crate::visibility::end_peek();
    crate::visibility::reveal();
    let (px, py) = cursor_pos();
    let s = sys_scale;
    let Some(f) = super::edit(|a| {
        let f = Fence::new(a.cfg.next_id(), a.t(T::Fence), (px - s(180), py - s(120), s(360), s(240)), a.cfg.look.clone());
        a.cfg.fences.push(f.clone());
        f
    }) else {
        return;
    };
    open(&f);
}

pub fn duplicate(id: u64) {
    let Some(f) = super::edit(|a| {
        let src = a.cfg.fence(id)?;
        let r = work_area(&RECT { left: src.x, top: src.y, right: src.x + src.w, bottom: src.y + src.h });
        a.cfg.duplicate(id, [r.left, r.top, r.right - r.left, r.bottom - r.top], sys_scale(16))
    })
    .flatten() else {
        return;
    };
    open(&f);
    crate::visibility::desks::sync();
}

fn open(f: &Fence) {
    let Some(h) = fence::create(f) else {
        return report::log(&format!("cannot create the window of fence {}", f.id));
    };
    let dirs: Vec<_> = f.tabs.iter().filter(|t| t.portal.is_none()).map(store::tab_dir).collect();
    report::failures(T::ErrSave, &store::ensure(&dirs));
    with(|a| a.views.push(View::new(h, f)));
    fence::bind(h);
}

pub fn rebuild() {
    crate::visibility::end_peek();
    crate::visibility::reveal();
    let Some((old, fences)) = with(|a| (std::mem::take(&mut a.views), a.cfg.fences.clone())) else {
        return;
    };
    for v in old {
        unsafe { DestroyWindow(v.hwnd).ok() };
    }
    fences.iter().for_each(open);
    crate::visibility::desks::sync();
}
