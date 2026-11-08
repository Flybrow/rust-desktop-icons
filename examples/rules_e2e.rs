mod support;

use serde_json::json;
use std::{
    env,
    fs::{self, File, FileTimes},
    os::windows::fs::FileTimesExt,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime},
};
use support::{app, chord, click, fences, poll, quit, rect, wait, windows, work};
use windows::Win32::UI::{
    HiDpi::GetDpiForWindow,
    Input::KeyboardAndMouse::{VK_CONTROL, VK_F10, VK_MENU, VK_SHIFT},
};

fn settled(p: &Path) {
    let old = SystemTime::now() - Duration::from_secs(120);
    let f = File::options().write(true).open(p).unwrap();
    f.set_times(FileTimes::new().set_modified(old).set_created(old)).unwrap();
}

fn drop_on_desktop(desk: &Path, name: &str) -> PathBuf {
    let p = desk.join(name);
    fs::write(&p, "x").unwrap();
    settled(&p);
    p
}

fn main() {
    let w = work();
    let tag = std::process::id();
    let desk = support::desk::desktop();
    let root = env::temp_dir().join(format!("rdi-rules-{tag}"));
    let data = root.join("RustDesktopIcons");
    let dir = |id: u64| data.join("fences").join(id.to_string());
    for id in 1..=3 {
        fs::create_dir_all(dir(id)).unwrap();
    }
    let fence = |id: u64, x: i32, title: &str, kinds: serde_json::Value| json!({"id": id, "x": w.left + x, "y": w.top + 650, "w": 280, "h": 200, "tabs": [{"id": id, "title": title, "kinds": kinds}]});
    let write = |auto: bool, kinds: bool| {
        let k = |v: serde_json::Value| if kinds { v } else { json!([]) };
        let cfg = json!({"lang": "en", "auto_update": false, "auto_sort": auto, "version": env!("CARGO_PKG_VERSION"),
            "settings_key": {"mods": 7, "vk": 121},
            "name_rules": [{"pattern": format!("bill-{tag}*.rdie2e"), "fence": 1, "tab": 1}],
            "fences": [fence(1, 20, "Bills", json!([])), fence(2, 320, "Inbox", k(json!(["Other"]))), fence(3, 620, "Pics", k(json!(["Images"])))]});
        fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    };
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| {
        support::verdict(ok, what, &mut fails);
    };
    let names =
        [format!("bill-{tag}-march.rdie2e"), format!("misc-{tag}.rdie2e"), format!("pic-{tag}.png"), format!("bill-{tag}-later.rdie2e")];

    write(true, true);
    let mut child = app(&root, &[], &[]);
    let pid = child.id();
    check(wait(10, || fences(pid).len() == 3), "three fences shown");
    thread::sleep(Duration::from_millis(1500));
    for n in &names[..3] {
        drop_on_desktop(&desk, n);
    }
    let landed = |n: &str, id: u64| dir(id).join(n).exists() && !desk.join(n).exists();
    check(wait(15, || landed(&names[0], 1)), "a new desktop file matching a name rule goes to its fence, before any type rule");
    check(wait(5, || landed(&names[2], 3)), "a new image goes to the fence with the Images rule");
    check(wait(5, || landed(&names[1], 2)), "a file no rule matches goes to the \"Everything else\" fence");
    quit(&mut child);

    write(false, false);
    let mut child = app(&root, &[], &[]);
    let pid = child.id();
    check(wait(10, || fences(pid).len() == 3), "fences shown with automatic sorting off");
    thread::sleep(Duration::from_millis(1500));
    drop_on_desktop(&desk, &names[3]);
    thread::sleep(Duration::from_millis(4000));
    check(desk.join(&names[3]).exists(), "with automatic sorting off, a new file stays on the desktop");
    chord(&[VK_CONTROL, VK_MENU, VK_SHIFT, VK_F10]);
    if let Some(s) = poll(3, || windows(pid, "RustDesktopIcons.Settings").first().copied()) {
        let (r, k) = (rect(s), unsafe { GetDpiForWindow(s) } as i32);
        click((r.left + 60 * k / 96, r.top + (56 + 12 + 40 * 5 + 20) * k / 96));
        thread::sleep(Duration::from_millis(400));
        click((r.left + (190 + 150) * k / 96, r.top + (56 + 46 + 66 * 4 + 20) * k / 96));
        check(wait(5, || landed(&names[3], 1)), "Sort the desktop now applies the rules to files already on the desktop");
    } else {
        check(false, "the settings window opens");
    }
    quit(&mut child);

    for n in &names {
        let _ = fs::remove_file(desk.join(n));
    }
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("RULES E2E OK");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
