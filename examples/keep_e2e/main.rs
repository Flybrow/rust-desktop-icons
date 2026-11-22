mod drag;
mod loose;
mod restart;
#[path = "../support/mod.rs"]
mod support;

use serde_json::{Value, json};
use std::{env, fs, path::PathBuf, process::Child, thread, time::Duration};
use support::{
    app,
    desk::{self, cover, desktop, flags, hidden, positions, removed, view},
    waited,
};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Com::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

const EXT: &str = "rdie2e";
const NAMES: [&str; 3] = ["rdi-e2e-1", "rdi-e2e-2", "rdi-e2e-3"];

fn file(name: &str) -> String {
    format!("{name}.{EXT}")
}

struct Run {
    root: PathBuf,
    desk: PathBuf,
    fails: Vec<String>,
}

impl Run {
    fn check(&mut self, ok: bool, what: &str) {
        support::verdict(ok, what, &mut self.fails);
    }

    fn data(&self) -> PathBuf {
        self.root.join("RustDesktopIcons")
    }

    fn tab_dir(&self) -> PathBuf {
        self.data().join("fences").join("1")
    }

    fn config(&self) -> Value {
        fs::read(self.data().join("config.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
    }

    fn pins(&self) -> Vec<String> {
        let c = self.config();
        c["fences"][0]["tabs"][0]["pins"].as_array().into_iter().flatten().filter_map(|p| p["path"].as_str()).map(String::from).collect()
    }

    fn set_keep(&self, on: bool) {
        let mut c = self.config();
        c["keep_on_desktop"] = json!(on);
        fs::write(self.data().join("config.json"), serde_json::to_vec_pretty(&c).unwrap()).unwrap();
    }

    fn start(&self) -> Child {
        let child = app(&self.root, &[("RDI_REAL_DESKTOP", "1"), ("RDI_TOP", "1")], &[]);
        desk::follow(child.id());
        child
    }

    fn stop(&self, mut app: Child) {
        if let Ok(t) = unsafe { FindWindowW(w!("RustDesktopIcons.Tray.dev"), None) } {
            let _ = unsafe { PostMessageW(Some(t), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
        if waited(10, || app.try_wait().ok().flatten().is_some()).is_none() {
            let _ = app.kill();
        }
    }
}

fn main() {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    let uncovered = support::free_desktop_spot(&[]).is_none();
    if uncovered {
        support::show_desktop();
    }
    let explorer = env::args().any(|a| a == "--restart-explorer");
    let mut r = Run { root: env::temp_dir().join(format!("rdi-e2e-{}", std::process::id())), desk: desktop(), fails: vec![] };
    let mut before = flags();
    fs::create_dir_all(r.tab_dir()).unwrap();
    for n in NAMES {
        fs::write(r.tab_dir().join(file(n)), "Rust Desktop Icons e2e test file").unwrap();
    }
    let cfg = json!({
        "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "keep_on_desktop": true,
        "fences": [{"id": 1, "x": 200, "y": 200, "w": 360, "h": 240, "tabs": [{"id": 1, "title": "E2E"}]}]
    });
    fs::write(r.data().join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let mut names: Vec<String> = NAMES.iter().map(|n| file(n)).collect();

    let app = r.start();
    thread::sleep(Duration::from_millis(500));
    let migrated = waited(15, || names.iter().all(|n| r.desk.join(n).exists()) && r.pins().len() == 3);
    r.check(migrated.is_some(), "enable: fence files moved to the Desktop folder and pinned");
    r.check(fs::read_dir(r.tab_dir()).map_or(true, |mut d| d.next().is_none()), "enable: fence folder emptied");
    r.check(waited(10, || hidden(&names) == 3).is_some(), "enable: icons hidden from the desktop");
    r.check(flags() == before, "auto-arrange and snap-to-grid untouched while active");
    r.check(cover().is_none() && removed(&names) == names.len(), "icons removed from the desktop view (no stack, no cover)");

    let _ = view().and_then(|v| v.cast::<IShellView>()).map(|v| unsafe { v.Refresh() });
    let back = waited(2, || hidden(&names) < 3).is_some();
    let rehid = waited(10, || hidden(&names) == 3);
    println!("      refresh: icons came back = {back}, hidden again after {} ms", rehid.map_or(-1, |m| m as i64));
    r.check(rehid.is_some(), "icons hidden again after desktop refresh (F5)");

    loose::run(&mut r, &names, app.id());

    let renamed = file("rdi-e2e-1-renamed");
    fs::rename(r.desk.join(&names[0]), r.desk.join(&renamed)).unwrap();
    names[0] = renamed.clone();
    let followed = waited(15, || r.pins().iter().any(|p| p.ends_with(&renamed)) && r.pins().len() == 3);
    r.check(followed.is_some(), "rename in Explorer: fence follows the file");
    r.check(waited(10, || hidden(&names) == 3).is_some(), "rename in Explorer: renamed icon stays hidden");
    drag::run(&mut r, &mut names, app.id());

    if explorer {
        before = restart::run(&mut r, &names);
    }

    r.stop(app);
    r.set_keep(false);
    let app = r.start();
    let restored = waited(15, || names.iter().all(|n| r.tab_dir().join(n).exists() && !r.desk.join(n).exists()));
    r.check(restored.is_some(), "disable: files moved back into the fence folder");
    r.check(waited(5, || r.pins().is_empty()).is_some(), "disable: no pins left");
    r.check(waited(5, || flags() == before).is_some(), "disable: desktop auto-arrange/snap restored");
    r.stop(app);

    for n in &names {
        let _ = fs::remove_file(r.desk.join(n));
    }
    let clean = waited(10, || !names.iter().any(|n| r.desk.join(n).exists()) && !positions().iter().any(|(p, _)| p.ends_with(EXT)));
    r.check(clean.is_some(), "cleanup: no test file left on the desktop");
    let _ = fs::remove_dir_all(&r.root);
    if uncovered {
        support::show_desktop();
    }
    if r.fails.is_empty() {
        println!("E2E OK");
    } else {
        println!("E2E FAILED: {}", r.fails.join("; "));
        std::process::exit(1);
    }
}
