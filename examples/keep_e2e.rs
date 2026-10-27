mod support;

use serde_json::{Value, json};
use std::{
    env, fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use windows::Win32::Foundation::{LPARAM, POINT, WPARAM};
use windows::Win32::System::Com::*;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Shell::Common::{ITEMIDLIST, STRRET};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

const EXT: &str = "rdie2e";
const NAMES: [&str; 3] = ["rdi-e2e-1", "rdi-e2e-2", "rdi-e2e-3"];
const MASK: u32 = (FWF_AUTOARRANGE.0 | FWF_SNAPTOGRID.0) as u32;

fn desktop() -> PathBuf {
    unsafe { PathBuf::from(SHGetKnownFolderPath(&FOLDERID_Desktop, KNOWN_FOLDER_FLAG(0), None).unwrap().to_string().unwrap()) }
}

fn file(name: &str) -> String {
    format!("{name}.{EXT}")
}

fn view() -> Result<IFolderView2> {
    unsafe {
        let windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL)?;
        let mut hwnd = 0;
        let disp =
            windows.FindWindowSW(&VARIANT::from(CSIDL_DESKTOP as i32), &VARIANT::default(), SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH)?;
        let browser: IShellBrowser = disp.cast::<IServiceProvider>()?.QueryService(&SID_STopLevelBrowser)?;
        browser.QueryActiveShellView()?.cast()
    }
}

fn positions() -> Vec<(String, POINT)> {
    let Ok(v) = view() else { return vec![] };
    let mut out = vec![];
    unsafe {
        let (Ok(folder), Ok(list)) = (v.GetFolder::<IShellFolder>(), v.Items::<IEnumIDList>(SVGIO_ALLVIEW)) else { return out };
        loop {
            let mut pidl: [*mut ITEMIDLIST; 1] = [std::ptr::null_mut()];
            if list.Next(&mut pidl, None).0 != 0 {
                break;
            }
            let mut sr = STRRET::default();
            let mut s = PWSTR::null();
            if folder.GetDisplayNameOf(pidl[0], SHGDN_FORPARSING, &mut sr).is_ok() && StrRetToStrW(&mut sr, Some(pidl[0]), &mut s).is_ok() {
                out.push((s.to_string().unwrap_or_default(), v.GetItemPosition(pidl[0]).unwrap_or_default()));
                CoTaskMemFree(Some(s.0 as _));
            }
            CoTaskMemFree(Some(pidl[0] as _));
        }
    }
    out
}

fn hidden(names: &[String]) -> usize {
    positions()
        .iter()
        .filter(|(p, pt)| pt.x < -10_000 && names.iter().any(|n| p.to_lowercase().ends_with(&format!("\\{}", n.to_lowercase()))))
        .count()
}

fn flags() -> u32 {
    view().and_then(|v| unsafe { v.GetCurrentFolderFlags() }).unwrap_or(0) & MASK
}

fn wait(secs: u64, mut f: impl FnMut() -> bool) -> Option<u128> {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(secs) {
        if f() {
            return Some(t.elapsed().as_millis());
        }
        thread::sleep(Duration::from_millis(10));
    }
    None
}

struct Run {
    root: PathBuf,
    fails: Vec<String>,
}

impl Run {
    fn check(&mut self, ok: bool, what: &str) {
        println!("[{}] {what}", if ok { "PASS" } else { "FAIL" });
        if !ok {
            self.fails.push(what.into());
        }
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
        let exe = env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("rust-desktop-icons.exe");
        Command::new(exe)
            .env("LOCALAPPDATA", &self.root)
            .env("RDI_REAL_DESKTOP", "1")
            .env("RDI_TOP", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("build the app first: cargo build")
    }

    fn stop(&self, mut app: Child) {
        if let Ok(t) = unsafe { FindWindowW(w!("RustDesktopIcons.Tray.dev"), None) } {
            let _ = unsafe { PostMessageW(Some(t), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
        if wait(10, || app.try_wait().ok().flatten().is_some()).is_none() {
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
    let migrated = wait(15, || names.iter().all(|n| desk.join(n).exists()) && r.pins().len() == 3);
    r.check(migrated.is_some(), "enable: fence files moved to the Desktop folder and pinned");
    r.check(fs::read_dir(r.tab_dir()).map_or(true, |mut d| d.next().is_none()), "enable: fence folder emptied");
    r.check(wait(10, || hidden(&names) == 3).is_some(), "enable: icons hidden from the desktop");
    r.check(flags() == 0, "auto-arrange and snap-to-grid off while active");

    let _ = view().and_then(|v| v.cast::<IShellView>()).map(|v| unsafe { v.Refresh() });
    let back = wait(1, || hidden(&names) < 3).is_some();
    let rehid = wait(10, || hidden(&names) == 3);
    println!("      refresh: icons came back = {back}, hidden again after {} ms", rehid.map_or(-1, |m| m as i64));
    r.check(rehid.is_some(), "icons hidden again after desktop refresh (F5)");

    let renamed = file("rdi-e2e-1-renamed");
    fs::rename(desk.join(&names[0]), desk.join(&renamed)).unwrap();
    names[0] = renamed.clone();
    let followed = wait(15, || r.pins().iter().any(|p| p.ends_with(&renamed)) && r.pins().len() == 3);
    r.check(followed.is_some(), "rename in Explorer: fence follows the file");
    r.check(wait(10, || hidden(&names) == 3).is_some(), "rename in Explorer: renamed icon stays hidden");

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
                    let left = wait(5, || r.pins().len() == 2);
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
                    let shown = wait(5, || positions().iter().any(|(p, pt)| p.ends_with(&out) && pt.x > -10_000));
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
        let t = wait(60, || hidden(&names) == names.len());
        println!("      explorer restart: hidden again after {} ms", t.map_or(-1, |m| m as i64));
        r.check(t.is_some(), "icons hidden again after Explorer restart");
    }

    r.stop(app);
    r.set_keep(false);
    let app = r.start();
    let restored = wait(15, || names.iter().all(|n| r.tab_dir().join(n).exists() && !desk.join(n).exists()));
    r.check(restored.is_some(), "disable: files moved back into the fence folder");
    r.check(wait(5, || r.pins().is_empty()).is_some(), "disable: no pins left");
    r.check(wait(5, || flags() == before).is_some(), "disable: desktop auto-arrange/snap restored");
    r.stop(app);

    for n in &names {
        let _ = fs::remove_file(desk.join(n));
    }
    let clean = wait(10, || !names.iter().any(|n| desk.join(n).exists()) && !positions().iter().any(|(p, _)| p.ends_with(EXT)));
    r.check(clean.is_some(), "cleanup: no test file left on the desktop");
    let _ = fs::remove_dir_all(&r.root);
    if r.fails.is_empty() {
        println!("E2E OK");
    } else {
        println!("E2E FAILED: {}", r.fails.join("; "));
        std::process::exit(1);
    }
}
