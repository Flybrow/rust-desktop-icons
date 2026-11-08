use crate::{
    app::{change, with},
    domain::hotkey::{self, Hotkey, Slot},
    report, search, tray,
    win::key_down,
};
use std::cell::Cell;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

thread_local! {
    static FAILED: Cell<[bool; Slot::ALL.len()]> = const { Cell::new([false; Slot::ALL.len()]) };
}

fn bind(s: Slot, k: Hotkey) -> bool {
    let Some(t) = tray::hwnd() else { return false };
    let ok = unsafe { RegisterHotKey(Some(t), s.id(), HOT_KEY_MODIFIERS(k.mods) | MOD_NOREPEAT, k.vk) }.is_ok();
    if !ok {
        report::log(&format!("shortcut {} is used by another program", k.label()));
    }
    ok
}

pub fn suspend() {
    if let Some(t) = tray::hwnd() {
        for s in Slot::ALL {
            let _ = unsafe { UnregisterHotKey(Some(t), s.id()) };
        }
    }
}

pub fn register() {
    suspend();
    let keys = with(|a| Slot::ALL.map(|s| a.cfg.key(s))).unwrap_or_default();
    FAILED.set(std::array::from_fn(|i| keys[i].is_some_and(|k| !bind(Slot::ALL[i], k))));
}

pub fn failed(s: Slot) -> bool {
    FAILED.get()[s as usize]
}

pub fn fired(id: i32) {
    match Slot::of(id) {
        Some(Slot::Peek) => super::toggle_peek(),
        Some(Slot::Search) => search::open(None),
        Some(Slot::HideAll) => super::toggle_hidden(),
        Some(Slot::NewFence) => crate::undo::step(crate::app::new_fence),
        Some(Slot::Settings) => crate::settings::open(),
        None => {}
    }
}

pub fn captured(vk: u16) -> Option<Option<Hotkey>> {
    let mods =
        [(VK_CONTROL, hotkey::CTRL), (VK_MENU, hotkey::ALT), (VK_SHIFT, hotkey::SHIFT), (VK_LWIN, hotkey::WIN), (VK_RWIN, hotkey::WIN)]
            .iter()
            .filter(|(k, _)| key_down(*k))
            .fold(0, |acc, (_, m)| acc | m);
    match VIRTUAL_KEY(vk) {
        VK_BACK | VK_DELETE if mods == 0 => Some(None),
        k => Some(Hotkey { mods, vk: k.0 as u32 }).filter(Hotkey::valid).map(Some),
    }
}

pub fn set(s: Slot, k: Option<Hotkey>) -> bool {
    let mut done = false;
    change(|c| done = c.set_key(s, k));
    register();
    done
}
