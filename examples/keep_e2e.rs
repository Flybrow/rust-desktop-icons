mod support;

use serde_json::{Value, json};
use std::{
    env, fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};
use support::{
    app,
    desk::{self, cover, desktop, flags, hidden, positions, screen, stacked, view},
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

fn restart_explorer() {
    let _ = Command::new("taskkill").args(["/f", "/im", "explorer.exe"]).stdout(Stdio::null()).stderr(Stdio::null()).status();
    let _ = Command::new("explorer.exe").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

fn main() {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    let uncovered = support::free_desktop_spot(&[]).is_none();
    if uncovered {
        support::show_desktop();
    }
    let explorer = env::args().any(|a| a == "--restart-explorer");
    let desk = desktop();
    let mut r = Run { root: env::temp_dir().join(format!("rdi-e2e-{}", std::process::id())), fails: vec![] };
    let before = flags();
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
    let migrated = waited(15, || names.iter().all(|n| desk.join(n).exists()) && r.pins().len() == 3);
    r.check(migrated.is_some(), "enable: fence files moved to the Desktop folder and pinned");
    r.check(fs::read_dir(r.tab_dir()).map_or(true, |mut d| d.next().is_none()), "enable: fence folder emptied");
    r.check(waited(10, || hidden(&names) == 3).is_some(), "enable: icons hidden from the desktop");
    r.check(flags() == 0, "auto-arrange and snap-to-grid off while active");
    r.check(cover().is_some(), "a cover window hides the stacked icons");

    let _ = view().and_then(|v| v.cast::<IShellView>()).map(|v| unsafe { v.Refresh() });
    let back = waited(1, || hidden(&names) < 3).is_some();
    let rehid = waited(10, || hidden(&names) == 3);
    println!("      refresh: icons came back = {back}, hidden again after {} ms", rehid.map_or(-1, |m| m as i64));
    r.check(rehid.is_some(), "icons hidden again after desktop refresh (F5)");

    let loose = file("rdi-e2e-loose");
    fs::write(desk.join(&loose), "Rust Desktop Icons e2e test file").unwrap();
    let spot = waited(10, || positions().iter().any(|(p, pt)| p.ends_with(&loose) && pt.x > -10_000 && !stacked(*pt)))
        .and_then(|_| positions().into_iter().find(|(p, _)| p.ends_with(&loose)))
        .map(|(_, pt)| screen(pt));
    match (spot, support::free_desktop_spot(&support::fences(app.id()))) {
        (Some(from), Some(to)) => {
            let watched = names.clone();
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let flag = stop.clone();
            let probe = thread::spawn(move || {
                unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
                let (mut least, mut seen) = (watched.len(), String::new());
                while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                    let n = hidden(&watched);
                    if n < least {
                        least = n;
                        let at: Vec<_> = positions().into_iter().filter(|(p, _)| p.ends_with(EXT)).map(|(p, pt)| (p, screen(pt))).collect();
                        seen = format!("cover {:?}, icons {at:?}", cover().map(|r| (r.left, r.top, r.right, r.bottom)));
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                (least, seen)
            });
            let saved = support::cursor();
            support::drag((from.0 + 20, from.1 + 20), to);
            support::point(saved);
            thread::sleep(Duration::from_millis(1500));
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            let (least, seen) = probe.join().unwrap();
            if least < names.len() {
                println!("      when shown: {seen}");
            }
            println!("      moving a loose icon: fewest hidden = {least}/{}", names.len());
            r.check(least == names.len(), "moving a loose desktop icon never shows the hidden icons");
        }
        _ => println!("[SKIP] moving a loose icon: icon or free spot not found"),
    }
    let _ = fs::remove_file(desk.join(&loose));

    let renamed = file("rdi-e2e-1-renamed");
    fs::rename(desk.join(&names[0]), desk.join(&renamed)).unwrap();
    names[0] = renamed.clone();
    let followed = waited(15, || r.pins().iter().any(|p| p.ends_with(&renamed)) && r.pins().len() == 3);
    r.check(followed.is_some(), "rename in Explorer: fence follows the file");
    r.check(waited(10, || hidden(&names) == 3).is_some(), "rename in Explorer: renamed icon stays hidden");

    match support::fences(app.id()).first().copied() {
        Some(f) => {
            let k = unsafe { windows::Win32::UI::HiDpi::GetDpiForSystem() } as i32;
            let (cell, cols) = (104 * k / 96, ((f.right - f.left) / (104 * k / 96)).max(1));
            let item = (f.left + ((f.right - f.left) - cols * cell) / 2 + cell / 2, f.top + (34 + 4) * k / 96 + cell / 2);
            match support::free_desktop_spot(&[f]) {
                Some(spot) => {
                    let saved = support::cursor();
                    let before: Vec<String> = r.pins();
                    support::drag(item, spot);
                    support::point(saved);
                    let left = waited(5, || r.pins().len() == 2);
                    let gone: Vec<String> = before.iter().filter(|p| !r.pins().contains(p)).cloned().collect();
                    let out = gone
                        .first()
                        .map(|p| std::path::Path::new(p).file_name().unwrap_or_default().to_string_lossy().into_owned())
                        .unwrap_or_default();
                    r.check(left.is_some(), "drag out to the desktop: the item leaves the fence");
                    r.check(!out.is_empty() && desk.join(&out).exists(), "drag out to the desktop: the file stays on the Desktop");
                    let dupes = fs::read_dir(&desk)
                        .into_iter()
                        .flatten()
                        .flatten()
                        .filter(|e| e.file_name().to_string_lossy().contains("rdi-e2e") && e.file_name().to_string_lossy().contains(" ("))
                        .count();
                    r.check(dupes == 0, "drag out to the desktop: no duplicate file");
                    r.check(support::windows(0, "OperationStatusWindow").is_empty(), "drag out to the desktop: no Explorer error dialog");
                    let shown = waited(5, || positions().iter().any(|(p, pt)| p.ends_with(&out) && pt.x > -10_000 && !stacked(*pt)));
                    r.check(shown.is_some(), "drag out to the desktop: its icon is shown again");
                    names.retain(|n| *n != out);
                    let _ = fs::remove_file(desk.join(&out));
                }
                None => println!("[SKIP] drag out to the desktop: no free desktop area visible"),
            }
        }
        None => r.check(false, "fence window found for drag test"),
    }

    if explorer {
        restart_explorer();
        let t = waited(60, || hidden(&names) == names.len());
        println!("      explorer restart: hidden again after {} ms", t.map_or(-1, |m| m as i64));
        r.check(t.is_some(), "icons hidden again after Explorer restart");
    }

    r.stop(app);
    r.set_keep(false);
    let app = r.start();
    let restored = waited(15, || names.iter().all(|n| r.tab_dir().join(n).exists() && !desk.join(n).exists()));
    r.check(restored.is_some(), "disable: files moved back into the fence folder");
    r.check(waited(5, || r.pins().is_empty()).is_some(), "disable: no pins left");
    r.check(waited(5, || flags() == before).is_some(), "disable: desktop auto-arrange/snap restored");
    r.stop(app);

    for n in &names {
        let _ = fs::remove_file(desk.join(n));
    }
    let clean = waited(10, || !names.iter().any(|n| desk.join(n).exists()) && !positions().iter().any(|(p, _)| p.ends_with(EXT)));
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
