mod support;

use serde_json::json;
use std::{env, fs, thread, time::Duration};
use support::{app, chord, fence_cfg, menu, quit, rect, right_click, wait, windows, work};
use windows::{
    Win32::{
        System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_BINARY, RegGetValueW},
        UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_D, VK_F4, VK_LEFT, VK_LWIN, VK_RIGHT},
    },
    core::w,
};

fn bytes(name: windows::core::PCWSTR) -> Vec<u8> {
    let key = w!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\VirtualDesktops");
    let mut size = 0u32;
    if unsafe { RegGetValueW(HKEY_CURRENT_USER, key, name, RRF_RT_REG_BINARY, None, None, Some(&mut size)) }.is_err() {
        return vec![];
    }
    let mut b = vec![0u8; size as usize];
    let _ = unsafe { RegGetValueW(HKEY_CURRENT_USER, key, name, RRF_RT_REG_BINARY, None, Some(b.as_mut_ptr().cast()), Some(&mut size)) };
    b
}

fn guid(b: &[u8]) -> String {
    let d1 = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    let (d2, d3) = (u16::from_le_bytes([b[4], b[5]]), u16::from_le_bytes([b[6], b[7]]));
    let tail: String = b[10..16].iter().map(|x| format!("{x:02X}")).collect();
    format!("{{{d1:08X}-{d2:04X}-{d3:04X}-{:02X}{:02X}-{tail}}}", b[8], b[9])
}

fn current() -> Option<String> {
    let b = bytes(w!("CurrentVirtualDesktop"));
    (b.len() >= 16).then(|| guid(&b))
}

fn ids() -> Vec<String> {
    bytes(w!("VirtualDesktopIDs")).as_chunks::<16>().0.iter().map(|c| guid(c)).collect()
}

fn go(target: &str) -> bool {
    let list = ids();
    for _ in 0..12 {
        let Some(now) = current() else { return false };
        if now == target {
            return true;
        }
        let (a, b) = (list.iter().position(|x| *x == now), list.iter().position(|x| x == target));
        chord(&[VK_LWIN, VK_CONTROL, if a > b { VK_LEFT } else { VK_RIGHT }]);
        thread::sleep(Duration::from_millis(900));
    }
    current().as_deref() == Some(target)
}

fn visible(pid: u32) -> Vec<i32> {
    windows(pid, "RustDesktopIcons.Fence").into_iter().map(|h| rect(h).left).collect()
}

fn main() {
    let w = work();
    let (before, original) = (ids().len(), current());
    chord(&[VK_LWIN, VK_CONTROL, VK_D]);
    thread::sleep(Duration::from_millis(1500));
    let (Some(extra), list) = (current(), ids()) else {
        println!("[FAIL] a temporary virtual desktop could be created");
        std::process::exit(1);
    };
    let home = original.unwrap_or_else(|| list.iter().find(|g| **g != extra).cloned().unwrap_or_default());
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| {
        support::verdict(ok, what, &mut fails);
    };
    check(list.len() == before.max(1) + 1 && go(&home), "a temporary virtual desktop is created, back on the first one");

    let root = env::temp_dir().join(format!("rdi-vdesk-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    for id in [1, 2] {
        fs::create_dir_all(data.join("fences").join(id.to_string())).unwrap();
    }
    let (xa, xb) = (w.left + 100, w.left + 500);
    let cfg = json!({"lang": "en", "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "fences": [
        {"id": 1, "x": xa, "y": w.top + 300, "w": 300, "h": 200, "desktop": home, "tabs": [{"id": 1, "title": "Home only"}]},
        {"id": 2, "x": xb, "y": w.top + 300, "w": 300, "h": 200, "tabs": [{"id": 2, "title": "Everywhere"}]}]});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let mut child = app(&root, &[], &[]);
    let pid = child.id();
    check(wait(10, || visible(pid).len() == 2), "both fences show on the desktop they belong to");
    thread::sleep(Duration::from_millis(800));

    go(&extra);
    check(wait(3, || visible(pid) == vec![xb]), "on another virtual desktop only the fence shown everywhere stays");
    if let Some(h) = windows(pid, "RustDesktopIcons.Fence").first().copied() {
        let r = rect(h);
        right_click((r.right - 30, r.bottom - 30));
        menu::open();
        menu::pick(&["Behavior", "Show on"]);
        let entries = menu::entries();
        check(
            entries.iter().any(|e| e.starts_with("All desktops")) && entries.iter().filter(|e| e.starts_with("Desktop ")).count() >= 2,
            "Show on lists all desktops and each virtual desktop",
        );
        let first = entries.iter().skip_while(|e| !e.starts_with("All desktops")).nth(1).cloned().unwrap_or_default();
        menu::pick(&[first.as_str()]);
        check(wait(3, || fence_cfg(&data, 2)["desktop"] == json!(ids()[0])), "Show on → a desktop binds the fence to it");
    }
    let first_is_home = ids()[0] == home;
    check(wait(3, || visible(pid).is_empty()), "a fence bound to another desktop hides at once");
    go(&home);
    check(wait(3, || visible(pid).len() == 1 + first_is_home as usize), "back on the first desktop its fences come back");
    quit(&mut child);

    go(&extra);
    chord(&[VK_LWIN, VK_CONTROL, VK_F4]);
    thread::sleep(Duration::from_millis(1200));
    go(&home);
    check(
        !ids().contains(&extra) && current().as_deref() == Some(home.as_str()),
        "the temporary desktop is closed, back on the original one",
    );
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("VDESK E2E OK");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
