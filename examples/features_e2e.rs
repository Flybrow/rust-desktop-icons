mod support;

use serde_json::json;
use std::{
    env,
    fs::{self, File},
    path::PathBuf,
    thread,
    time::{Duration, SystemTime},
};
use support::{app, capture, chord, double_click, drag, fences, point, press, quit, rect, type_text, wait, windows, work};
use windows::Win32::{
    System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
    UI::{
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::{VK_A, VK_BACK, VK_CONTROL, VK_F2, VK_F10, VK_MENU, VK_RETURN, VK_SHIFT},
    },
};

const PAGES: usize = 8;

fn main() {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    let shots = env::args().skip_while(|a| a != "--shots").nth(1).map(PathBuf::from);
    let w = work();
    let root = env::temp_dir().join(format!("rdi-features-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    let (docs, archive) = (data.join("fences").join("1"), data.join("fences").join("2"));
    for d in [&docs, &archive, &data.join("fences").join("3")] {
        fs::create_dir_all(d).unwrap();
    }
    fs::create_dir_all(docs.join("Sub")).unwrap();
    let browse = data.join("fences").join("6").join("Sub");
    fs::create_dir_all(data.join("fences").join("4")).unwrap();
    fs::write(data.join("fences").join("4").join("first.txt"), "x").unwrap();
    fs::create_dir_all(&browse).unwrap();
    fs::write(browse.join("inner.txt"), "x").unwrap();
    for f in ["old.txt", "new.txt"] {
        fs::write(docs.join(f), "x").unwrap();
    }
    let old = SystemTime::now() - Duration::from_secs(40 * 86_400);
    File::options().write(true).open(docs.join("old.txt")).unwrap().set_modified(old).unwrap();
    let fence = |id: u64, x: i32, title: &str, extra: serde_json::Value| {
        let mut f = json!({"id": id, "x": w.left + x, "y": w.top + 120, "w": 300, "h": 220, "tabs": [{"id": id, "title": title}]});
        f.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        f
    };
    let expire = json!({"tabs": [{"id": 1, "title": "Docs", "expire": {"days": 30, "fence": 2, "tab": 2}}], "group": 1});
    let cfg = json!({"auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"),
        "settings_key": {"mods": 7, "vk": 121},
        "name_rules": [{"pattern": "facture*.pdf", "fence": 2, "tab": 2}],
        "fences": [fence(1, 20, "Docs", expire), fence(2, 330, "Archive", json!({})), fence(3, 1320, "Mate", json!({"group": 1})),
            fence(4, 20, "Browse", json!({"y": w.top + 620, "tab": 2,
                "tabs": [{"id": 4, "title": "One"}, {"id": 5, "title": "Two"}, {"id": 6, "title": "Three"}]}))]});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let mut app = app(&root, &[("RDI_TOP", "1"), ("RDI_AGE_MS", "1500")], &[]);
    let pid = app.id();
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| {
        support::verdict(ok, what, &mut fails);
    };
    check(wait(10, || fences(pid).len() == 4), "four fences shown");
    thread::sleep(Duration::from_millis(800));

    check(wait(10, || archive.join("old.txt").exists()), "an item older than the age rule moves to the destination fence");
    check(docs.join("new.txt").exists() && docs.join("Sub").exists(), "recent items stay where they are");

    let at = |x: i32| fences(pid).into_iter().find(|r| (r.left - (w.left + x)).abs() < 40 && (r.top - (w.top + 120)).abs() < 40);
    let (Some(a0), Some(c0)) = (at(20), at(1320)) else {
        check(false, "fences found at their configured places");
        quit(&mut app);
        std::process::exit(1);
    };
    drag((a0.left + 60, a0.top + 12), (a0.left + 140, a0.top + 72));
    thread::sleep(Duration::from_millis(600));
    let moved = |r: windows::Win32::Foundation::RECT| (r.left - c0.left, r.top - c0.top);
    let c1 = fences(pid).into_iter().find(|r| (r.left - c0.left - 80).abs() < 30 && (r.top - c0.top - 60).abs() < 30);
    check(c1.is_some(), &format!("a grouped fence follows the dragged one ({:?})", c1.map(moved)));
    thread::sleep(Duration::from_millis(500));
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(data.join("config.json")).unwrap()).unwrap();
    let x3 = saved["fences"].as_array().unwrap().iter().find(|f| f["id"] == 3).map(|f| f["x"].as_i64().unwrap_or(0));
    check(x3.is_some_and(|x| (x as i32 - (w.left + 1320) - 80).abs() < 30), "the grouped fence's new place is saved");

    let fence4 = windows(pid, "RustDesktopIcons.Fence").into_iter().find(|h| (rect(*h).top - (w.top + 620)).abs() < 40);
    if let Some(h) = fence4 {
        let (b, k) = (rect(h), unsafe { GetDpiForWindow(h) } as i32);
        let cell = (b.left + 60 * k / 96, b.top + 75 * k / 96);
        double_click(cell);
        thread::sleep(Duration::from_millis(800));
        point(cell);
        press(true);
        press(false);
        thread::sleep(Duration::from_millis(300));
        chord(&[VK_F2]);
        thread::sleep(Duration::from_millis(300));
        type_text("renamed");
        chord(&[VK_RETURN]);
        let renamed = || browse.join("renamed.txt").exists() || browse.join("renamed").exists();
        check(wait(3, renamed), "a double-clicked folder opens inside the fence, where its items can be renamed");
        chord(&[VK_BACK]);
        thread::sleep(Duration::from_millis(500));
        point(cell);
        press(true);
        press(false);
        thread::sleep(Duration::from_millis(300));
        chord(&[VK_F2]);
        thread::sleep(Duration::from_millis(300));
        chord(&[VK_CONTROL, VK_A]);
        type_text("Sub2");
        chord(&[VK_RETURN]);
        check(wait(3, || browse.with_file_name("Sub2").is_dir()), "Backspace goes back to the fence's own items");
        let bar = b.top + 12 * k / 96;
        let rename = |to: &str| {
            point(cell);
            press(true);
            press(false);
            thread::sleep(Duration::from_millis(300));
            chord(&[VK_F2]);
            thread::sleep(Duration::from_millis(300));
            chord(&[VK_CONTROL, VK_A]);
            type_text(to);
            chord(&[VK_RETURN]);
        };
        let click = |x: i32| {
            point((x, bar));
            press(true);
            press(false);
            thread::sleep(Duration::from_millis(600));
        };
        double_click(cell);
        thread::sleep(Duration::from_millis(800));
        for x in (b.left + 20..b.right - 20).rev().step_by(10) {
            point((x, bar));
            thread::sleep(Duration::from_millis(15));
        }
        click(b.left + 30);
        rename("Sub3");
        check(wait(3, || browse.with_file_name("Sub3").is_dir()), "with several tabs, the back arrow stays under the mouse and goes back");
        double_click(cell);
        thread::sleep(Duration::from_millis(800));
        click((b.left + b.right) / 2);
        click(b.right - 30);
        rename("Sub4");
        check(wait(3, || browse.with_file_name("Sub4").is_dir()), "leaving a tab while in a folder brings that tab back to its own items");
        if let Some(dir) = &shots {
            capture(&b, &dir.join("browse_back.bmp"));
        }
    } else {
        check(false, "the browse fence is shown");
    }

    chord(&[VK_CONTROL, VK_MENU, VK_SHIFT, VK_F10]);
    check(wait(3, || windows(pid, "RustDesktopIcons.Settings").len() == 1), "the settings shortcut opens the settings");
    if let Some(s) = windows(pid, "RustDesktopIcons.Settings").first().copied() {
        let r = rect(s);
        let k = unsafe { GetDpiForWindow(s) } as i32;
        let sc = |v: i32| v * k / 96;
        for page in 0..PAGES {
            point((r.left + sc(60), r.top + sc(56 + 12 + 40 * page as i32 + 20)));
            thread::sleep(Duration::from_millis(150));
            press(true);
            press(false);
            thread::sleep(Duration::from_millis(400));
            if let Some(dir) = &shots {
                capture(&rect(s), &dir.join(format!("settings_{page}.bmp")));
            }
        }
        check(windows(pid, "RustDesktopIcons.Settings").len() == 1, "every settings page opens without closing the window");
    }

    quit(&mut app);
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("all checks passed");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
