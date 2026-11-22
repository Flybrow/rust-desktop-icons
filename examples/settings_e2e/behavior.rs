use super::{DESCRIBED, HEADER, HIDE, NEW, NOTE, RESET, SETTINGS, Ui, f};
use crate::support::{chord, fences, rect, wait};
use serde_json::json;
use std::{thread, time::Duration};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;

pub fn behave(u: &mut Ui) {
    let behave = [HEADER, NOTE, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, NOTE];
    u.page(3);
    for (i, key) in [(2, "chameleon"), (3, "auto_height"), (4, "browse"), (5, "home")] {
        let before = u.cfg()["look"][key].as_bool().unwrap_or(false);
        u.row(&behave, i);
        let changed = u.until(|c| c["look"][key] == !before && f(c, 1, key) == !before);
        u.row(&behave, i);
        u.check(changed && u.until(|c| c["look"][key] == before), &format!("Behavior → {key} toggles for every fence"));
    }
    u.row(&behave, 6);
    u.pick(&["Instant"]);
    u.check(u.until(|c| c["roll_ms"] == 0), "Behavior → Unroll speed is saved");
}

pub fn desk(u: &mut Ui) {
    let desk = [HEADER, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED];
    u.page(4);
    u.row(&desk, 3);
    let on = u.until(|c| c["dblclick_hide"] == true);
    u.row(&desk, 3);
    u.check(on && u.until(|c| c["dblclick_hide"] == false), "Desktop → Hide all on double-click toggles");
    u.row(&desk, 4);
    let off = u.until(|c| c["hide_desktop"] == false);
    u.row(&desk, 4);
    u.check(off && u.until(|c| c["hide_desktop"] == true), "Desktop → Hide all: desktop icons too toggles");
    u.row(&desk, 2);
    u.pick(&["Don't adjust"]);
    u.check(u.until(|c| c["screen_adapt"] == "Keep"), "Desktop → When the screen changes is saved");
}

pub fn keys(u: &mut Ui) {
    let mut keys = vec![HEADER, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED + RESET];
    u.page(6);
    u.row(&keys, 3);
    chord(&HIDE);
    u.check(u.until(|c| c["hide_key"] == json!({"mods": 7, "vk": 120})), "Shortcuts → a pressed combination becomes the hide shortcut");
    keys[3] += RESET;
    u.row(&keys, 4);
    chord(&SETTINGS);
    thread::sleep(Duration::from_millis(300));
    chord(&[VK_ESCAPE]);
    u.check(u.cfg()["new_key"].is_null(), "a combination already used by another shortcut is refused");
    u.row(&keys, 4);
    chord(&NEW);
    u.check(u.until(|c| c["new_key"] == json!({"mods": 7, "vk": 119})), "Shortcuts → new fence shortcut is saved");
    keys[4] += RESET;
    chord(&HIDE);
    let hidden = wait(3, || fences(u.pid).is_empty());
    chord(&HIDE);
    u.check(hidden && wait(3, || fences(u.pid).len() == 2), "the hide shortcut hides the fences, then brings them back");
    chord(&NEW);
    u.check(wait(3, || fences(u.pid).len() == 3), "the new fence shortcut creates a fence");
    thread::sleep(Duration::from_millis(400));
    let r = rect(u.h);
    let over = unsafe {
        windows::Win32::UI::WindowsAndMessaging::WindowFromPoint(windows::Win32::Foundation::POINT {
            x: (r.left + r.right) / 2,
            y: (r.top + r.bottom) / 2,
        })
    };
    let top = unsafe { windows::Win32::UI::WindowsAndMessaging::GetAncestor(over, windows::Win32::UI::WindowsAndMessaging::GA_ROOT) };
    u.check(top == u.h, "a new fence stays behind the open windows");
    u.reset(&keys, 3);
    u.check(u.until(|c| c["hide_key"].is_null()), "Reset to default clears a changed shortcut");
}
