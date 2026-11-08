use super::{edit, moveto, view};
use crate::{
    app::with,
    i18n::T,
    shell::menu::{self, Pick},
    win::key_down,
};
use std::path::PathBuf;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::Input::KeyboardAndMouse::VK_SHIFT,
};

const HERE: usize = 0xFFF;

pub(super) fn show(h: HWND, paths: &[PathBuf], at: (i32, i32)) {
    let extended = key_down(VK_SHIFT);
    let Some(native) = menu::build(paths, extended) else { return };
    let (sub, mut targets) = moveto::submenu(h);
    if let Some(label) = with(|a| a.t(T::MoveTo)) {
        native.prepend(label, sub);
    }
    let folder = paths.first().filter(|p| paths.len() == 1 && super::browse::openable(p)).cloned();
    if let Some(label) = with(|a| a.t(T::OpenInFence)).filter(|_| folder.is_some()) {
        native.prepend_item(label, HERE);
    }
    view(h, |v| v.native = Some(native.cm.clone()));
    let pick = native.track(super::host::for_menu(h), at);
    view(h, |v| v.native = None);
    match pick {
        Pick::Own(HERE) => folder.iter().for_each(|p| super::browse::enter(h, p)),
        Pick::Own(k) if k < targets.len() => moveto::run(h, paths.to_vec(), targets.swap_remove(k)),
        Pick::Shell(id) if native.verb(id).eq_ignore_ascii_case("rename") => paths.first().iter().for_each(|p| edit::item(h, p)),
        Pick::Shell(id) => native.invoke(id, h, at),
        _ => {}
    }
}

pub(super) fn forward(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
    let cm = with(|a| a.view(h)?.native.clone()).flatten()?;
    menu::forward(&cm, msg, wp, lp)
}
