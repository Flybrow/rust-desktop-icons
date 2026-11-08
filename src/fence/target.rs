use super::{drop, items, peek, system, tabs, to_client};
use crate::{app::with, domain::system::Sys, shell, store};
use std::{cell::RefCell, path::PathBuf};
use windows::{
    Win32::{
        Foundation::{HWND, POINT, POINTL},
        System::{
            Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, IDataObject},
            Ole::*,
            SystemServices::MODIFIERKEYS_FLAGS,
        },
        UI::Shell::{CLSID_DragDropHelper, IDropTargetHelper},
    },
    core::{Ref, Result, implement},
};

#[implement(IDropTarget)]
struct Target {
    hwnd: isize,
    helper: Option<IDropTargetHelper>,
    data: RefCell<Option<(IDataObject, Vec<PathBuf>, bool)>>,
    inner: RefCell<Option<(PathBuf, Option<IDropTarget>)>>,
    checked: RefCell<Option<(PathBuf, bool)>>,
}

impl Target {
    fn h(&self) -> HWND {
        HWND(self.hwnd as _)
    }

    fn hover(&self, pt: &POINTL) {
        peek::enter(self.h());
        if let Some(i) = tabs::at(self.h(), to_client(self.h(), (pt.x, pt.y))) {
            tabs::select(self.h(), i);
        }
    }

    fn effect(allowed: DROPEFFECT) -> DROPEFFECT {
        [DROPEFFECT_MOVE, DROPEFFECT_COPY, DROPEFFECT_LINK].into_iter().find(|e| allowed.0 & e.0 != 0).unwrap_or(DROPEFFECT_NONE)
    }

    fn leave_inner(&self) {
        if let Some((_, Some(t))) = self.inner.borrow_mut().take() {
            let _ = unsafe { t.DragLeave() };
        }
    }

    fn enter_item(&self, p: PathBuf, keys: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> bool {
        let data = self.data.borrow().as_ref().map(|(d, _, _)| d.clone());
        let t = data.and_then(|d| {
            let t = shell::drop_target(&p)?;
            let mut e = unsafe { *effect };
            let ok = unsafe { t.DragEnter(&d, keys, *pt, &mut e) }.is_ok() && e != DROPEFFECT_NONE;
            if !ok {
                let _ = unsafe { t.DragLeave() };
                return None;
            }
            unsafe { *effect = e };
            Some(t)
        });
        let entered = t.is_some();
        *self.inner.borrow_mut() = Some((p, t));
        entered
    }

    fn accepts(&self, p: &PathBuf) -> bool {
        if let Some((_, ok)) = self.checked.borrow().as_ref().filter(|(q, _)| q == p) {
            return *ok;
        }
        let ok = Sys::of(p).is_none()
            && self
                .data
                .borrow()
                .as_ref()
                .is_none_or(|(_, dragged, internal)| !dragged.iter().any(|x| shell::same_path(x, p)) && (!internal || p.is_dir()));
        *self.checked.borrow_mut() = Some((p.clone(), ok));
        ok
    }

    fn aim(&self, keys: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> bool {
        let at = to_client(self.h(), (pt.x, pt.y));
        let item = items::item_at(self.h(), at).filter(|p| self.accepts(p));
        let same = matches!((&*self.inner.borrow(), &item), (Some((a, _)), Some(b)) if a == b);
        let delegated = if same {
            let inner = self.inner.borrow().as_ref().and_then(|(_, t)| t.clone());
            inner.is_some_and(|t| {
                let _ = unsafe { t.DragOver(keys, *pt, effect) };
                true
            })
        } else {
            self.leave_inner();
            item.is_some_and(|p| self.enter_item(p, keys, pt, effect))
        };
        items::hover(self.h(), if delegated { items::index_at(self.h(), at) } else { None });
        delegated
    }

    fn show(&self, pt: &POINTL, effect: DROPEFFECT, data: Option<&IDataObject>) {
        let Some(helper) = &self.helper else { return };
        let p = POINT { x: pt.x, y: pt.y };
        let _ = match data {
            Some(d) => unsafe { helper.DragEnter(self.h(), d, &p, effect) },
            None => unsafe { helper.DragOver(&p, effect) },
        };
    }
}

impl IDropTarget_Impl for Target_Impl {
    fn DragEnter(&self, data: Ref<IDataObject>, keys: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        self.checked.borrow_mut().take();
        *self.data.borrow_mut() = data.as_ref().map(|d| {
            let files = shell::files(d);
            let ours = |p: &PathBuf| store::in_fences(p) || with(|a| a.cfg.is_pinned(p)) == Some(true);
            let internal = !files.is_empty() && files.iter().all(ours);
            (d.clone(), files, internal)
        });
        if !self.aim(keys, pt, effect) {
            unsafe { *effect = Target::effect(*effect) };
        }
        self.show(pt, unsafe { *effect }, data.as_ref());
        self.hover(pt);
        Ok(())
    }

    fn DragOver(&self, keys: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        if !self.aim(keys, pt, effect) {
            unsafe { *effect = Target::effect(*effect) };
        }
        self.show(pt, unsafe { *effect }, None);
        self.hover(pt);
        Ok(())
    }

    fn DragLeave(&self) -> Result<()> {
        self.leave_inner();
        self.data.borrow_mut().take();
        items::hover(self.h(), None);
        if let Some(helper) = &self.helper {
            let _ = unsafe { helper.DragLeave() };
        }
        Ok(())
    }

    fn Drop(&self, data: Ref<IDataObject>, keys: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        crate::undo::step(|| self.drop_inner(data, keys, pt, effect))
    }
}

impl Target_Impl {
    fn drop_inner(&self, data: Ref<IDataObject>, keys: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        if let (Some(helper), Some(d)) = (&self.helper, data.as_ref()) {
            let _ = unsafe { helper.Drop(d, &POINT { x: pt.x, y: pt.y }, *effect) };
        }
        let inner = self.inner.borrow_mut().take().and_then(|(_, t)| t);
        self.data.borrow_mut().take();
        items::hover(self.h(), None);
        if let (Some(t), Some(d)) = (inner, data.as_ref()) {
            let _ = unsafe { t.Drop(d, keys, *pt, effect) };
            items::reload_all();
            return Ok(());
        }
        unsafe { *effect = DROPEFFECT_NONE };
        let at = to_client(self.h(), (pt.x, pt.y));
        if !system::dropped(self.h(), data.as_ref(), at) {
            drop::files(self.h(), data.as_ref().map(shell::files).unwrap_or_default(), at);
        }
        Ok(())
    }
}

pub(super) fn register(h: HWND) {
    let helper = unsafe { CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER) }.ok();
    let target: IDropTarget =
        Target { hwnd: h.0 as isize, helper, data: RefCell::new(None), inner: RefCell::new(None), checked: RefCell::new(None) }.into();
    if let Err(e) = unsafe { RegisterDragDrop(h, &target) } {
        crate::report::log(&format!("drag and drop unavailable on a fence: {e}"));
    }
}

pub(super) fn revoke(h: HWND) {
    let _ = unsafe { RevokeDragDrop(h) };
}
