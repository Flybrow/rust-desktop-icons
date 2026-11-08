mod support;

use serde_json::{Value, json};
use std::{env, fs, path::Path};
use support::{answer, app, fences, quit, saved, wait, windows, work};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_RETURN;

fn fence(id: u64, x: i32, y: i32) -> Value {
    json!({"id": id, "x": x, "y": y, "w": 300, "h": 200, "tabs": [{"id": id, "title": format!("F{id}")}]})
}

fn config(fences: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec_pretty(
        &json!({"lang": "en", "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "fences": fences}),
    )
    .unwrap()
}

fn recovered(data: &Path) -> Vec<(u64, String)> {
    saved(data)["fences"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|f| f["tabs"].as_array().cloned().unwrap_or_default())
        .filter(|t| t["title"].as_str().is_some_and(|s| s.starts_with("Recovered files")))
        .map(|t| (t["id"].as_u64().unwrap_or(0), t["title"].as_str().unwrap_or("").to_string()))
        .collect()
}

fn main() {
    let w = work();
    let (x, y) = (w.left + 300, w.top + 200);
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| support::verdict(ok, what, &mut fails);
    let tag = std::process::id();

    let root = env::temp_dir().join(format!("rdi-recover-{tag}"));
    let data = root.join("RustDesktopIcons");
    let dir = |id: &str| data.join("fences").join(id);
    for (id, file) in [("1", Some("kept.txt")), ("5", Some("orphan.txt")), ("6", None), ("notes", Some("x.txt"))] {
        fs::create_dir_all(dir(id)).unwrap();
        if let Some(f) = file {
            fs::write(dir(id).join(f), "x").unwrap();
        }
    }
    fs::write(data.join("config.json"), config(vec![fence(1, x, y)])).unwrap();
    let mut child = app(&root, &[("RDI_TOP", "1")], &[]);
    check(wait(10, || fences(child.id()).len() == 2), "a folder no fence uses comes back in its own fence, drawn at once");
    check(wait(3, || recovered(&data) == vec![(5, "Recovered files".to_string())]), "the recovered fence shows that folder as a tab");
    check(dir("5").join("orphan.txt").exists() && dir("1").join("kept.txt").exists(), "no file moves while recovering");
    quit(&mut child);
    let mut child = app(&root, &[("RDI_TOP", "1")], &[]);
    check(wait(10, || fences(child.id()).len() == 2) && recovered(&data).len() == 1, "the next start recovers nothing twice");
    quit(&mut child);
    let _ = fs::remove_dir_all(&root);

    let root = env::temp_dir().join(format!("rdi-damaged-{tag}"));
    let data = root.join("RustDesktopIcons");
    fs::create_dir_all(data.join("fences").join("1")).unwrap();
    fs::create_dir_all(data.join("fences").join("2")).unwrap();
    fs::write(data.join("config.json"), "{broken").unwrap();
    fs::write(data.join("config.json.bak"), config(vec![fence(1, x, y), fence(2, x + 350, y)])).unwrap();
    let mut child = app(&root, &[("RDI_TOP", "1")], &[]);
    let pid = child.id();
    check(wait(15, || fences(pid).len() == 2), "the fences come back from the last good save, before the warning is closed");
    check(
        wait(10, || !windows(pid, "#32770").is_empty()) && answer(pid, &[VK_RETURN]) && wait(3, || windows(pid, "#32770").is_empty()),
        "a damaged config is reported once",
    );
    quit(&mut child);
    let bad: Vec<String> = fs::read_dir(&data)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("config.json.bad-"))
        .filter_map(|e| fs::read_to_string(e.path()).ok())
        .collect();
    check(bad == vec!["{broken".to_string()], "the damaged file is kept aside, untouched");
    check(saved(&data)["fences"].as_array().is_some_and(|a| a.len() == 2), "the restored config is saved again");
    let _ = fs::remove_dir_all(&root);

    let root = env::temp_dir().join(format!("rdi-bom-{tag}"));
    let data = root.join("RustDesktopIcons");
    fs::create_dir_all(data.join("fences").join("1")).unwrap();
    fs::write(data.join("config.json"), [b"\xEF\xBB\xBF".as_slice(), &config(vec![fence(1, x, y)])].concat()).unwrap();
    let mut child = app(&root, &[("RDI_TOP", "1")], &[]);
    let pid = child.id();
    check(
        wait(10, || fences(pid).len() == 1) && windows(pid, "#32770").is_empty(),
        "a config saved with a BOM (Notepad) loads without warning",
    );
    quit(&mut child);
    let _ = fs::remove_dir_all(&root);

    if fails.is_empty() {
        println!("RECOVER E2E OK");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
