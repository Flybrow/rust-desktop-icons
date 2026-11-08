mod support;

use serde_json::json;
use std::{env, fs, thread, time::Duration};
use support::app;

fn main() {
    let tag = std::process::id();
    let desk = support::desk::desktop();
    let root = env::temp_dir().join(format!("rdi-uninstall-{tag}"));
    let data = root.join("RustDesktopIcons");
    let (known, unknown) = (data.join("fences").join("1"), data.join("fences").join("99"));
    fs::create_dir_all(&known).unwrap();
    fs::create_dir_all(unknown.join(format!("folder-{tag}.rdie2e"))).unwrap();
    let (file, folder) = (format!("file-{tag}.rdie2e"), format!("folder-{tag}.rdie2e"));
    fs::write(known.join(&file), "x").unwrap();
    fs::write(unknown.join(&folder).join("inside.txt"), "x").unwrap();
    let cfg = json!({"lang": "en", "auto_update": false, "version": env!("CARGO_PKG_VERSION"),
        "fences": [{"id": 1, "x": 100, "y": 100, "w": 300, "h": 200, "tabs": [{"id": 1, "title": "A"}]}]});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();

    let mut child = app(&root, &[], &["--uninstall"]);
    let mut done = None;
    for _ in 0..100 {
        if let Ok(Some(s)) = child.try_wait() {
            done = Some(s);
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| {
        support::verdict(ok, what, &mut fails);
    };
    check(done.is_some_and(|s| s.success()), "--uninstall finishes on its own");
    check(desk.join(&file).exists(), "items of a fence go back to the desktop");
    check(desk.join(&folder).join("inside.txt").exists(), "a fence folder missing from the config is emptied too");
    check(!data.exists(), "the app data folder is removed once every fence is empty");
    if done.is_none() {
        let _ = child.kill();
    }
    let _ = fs::remove_file(desk.join(&file));
    let _ = fs::remove_dir_all(desk.join(&folder));
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("UNINSTALL E2E OK");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
