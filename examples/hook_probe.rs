mod support;

use rdi_hook::{command, map::Map, shared::*};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    thread,
    time::{Duration, Instant},
};
use support::desk;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::{Com::*, LibraryLoader::*},
        UI::{Shell::IShellView, WindowsAndMessaging::*},
    },
    core::*,
};

fn shown(name: &str) -> bool {
    desk::positions().iter().any(|(p, _)| p.to_lowercase().ends_with(name))
}

fn defview() -> HWND {
    unsafe { GetParent(desk::listview()).unwrap_or_default() }
}

fn explorer() -> u32 {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(desk::listview(), Some(&mut pid)) };
    pid
}

type Proc = unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT;

struct Hooks(Vec<HHOOK>);

impl Hooks {
    fn install() -> Self {
        let dll = std::env::current_exe().unwrap().with_file_name("rdi_hook.dll");
        unsafe {
            let m = LoadLibraryW(&HSTRING::from(dll.as_os_str())).expect("load rdi_hook.dll");
            let get = GetProcAddress(m, s!("rdi_get_msg")).unwrap();
            let tid = GetWindowThreadProcessId(defview(), None);
            let b = SetWindowsHookExW(
                WH_GETMESSAGE,
                Some(std::mem::transmute::<unsafe extern "system" fn() -> isize, Proc>(get)),
                Some(m.into()),
                tid,
            )
            .unwrap();
            println!("hooks installed on explorer thread {tid}");
            Self(vec![b])
        }
    }
}

impl Drop for Hooks {
    fn drop(&mut self) {
        for h in &self.0 {
            let _ = unsafe { UnhookWindowsHookEx(*h) };
        }
    }
}

fn send(map: &Map) -> u128 {
    let t = Instant::now();
    if !command(defview(), map, Duration::from_secs(5)) {
        println!("  command failed: stage={} hr={:#x} errors={}", map.get(STAGE), map.get(HR), map.get(ERRORS));
    }
    t.elapsed().as_millis()
}

fn stats(map: &Map, label: &str) {
    let g = |i| map.get(i);
    println!(
        "  [{label}] passes={} removed={} added={} inserts={} refreshes={} errors={} items={} hidden={} last_pass={}us filtered={} installs={} stage={} sweeps={}",
        g(PASSES),
        g(REMOVED),
        g(ADDED),
        g(INSERTS),
        g(REFRESHES),
        g(ERRORS),
        g(ITEMS),
        g(HIDDEN),
        g(MICROS),
        g(FILTERED),
        g(INSTALLS),
        g(STAGE),
        g(SWEEPS)
    );
}

fn watch(name: &str, ms: u64) -> (bool, bool) {
    let t = Instant::now();
    let mut ever = false;
    while t.elapsed() < Duration::from_millis(ms) {
        ever |= shown(name);
        thread::sleep(Duration::from_millis(10));
    }
    (ever, shown(name))
}

fn refresh() {
    if let Ok(v) = desk::view().and_then(|v| v.cast::<IShellView>()) {
        let _ = unsafe { v.Refresh() };
    }
}

fn file(name: &str) -> PathBuf {
    let p = desk::desktop().join(name);
    fs::write(&p, b"probe").unwrap();
    p
}

fn key_of(p: &std::path::Path) -> String {
    key(&p.to_string_lossy())
}

fn child() {
    let map = Map::create(std::process::id()).unwrap();
    map.write(&[key_of(&desk::desktop().join("hidden-a.rdie2e"))]);
    let _h = Hooks::install();
    send(&map);
    println!("child: hidden-a shown={}", shown("hidden-a.rdie2e"));
    unsafe { windows::Win32::System::Threading::ExitProcess(0) };
}

fn main() {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    if std::env::args().any(|a| a == "--child") {
        return child();
    }
    let n: usize = std::env::var("PROBE_N").ok().and_then(|v| v.parse().ok()).unwrap_or(300);
    let a = file("hidden-a.rdie2e");
    let keep = file("visible-a.rdie2e");
    let many: Vec<PathBuf> = (0..n).map(|i| file(&format!("bulk-{i:04}.rdie2e"))).collect();
    support::waited(30, || shown("hidden-a.rdie2e") && shown(&format!("bulk-{:04}.rdie2e", n - 1)));
    println!("desktop items: {}, explorer pid {}", desk::positions().len(), explorer());
    let d = desk::desktop();
    let list = |names: &[&str]| names.iter().map(|n| key_of(&d.join(n))).collect::<Vec<_>>();
    let map = Map::create(std::process::id()).unwrap();
    map.write(&list(&["hidden-a.rdie2e", "hidden-b.rdie2e", "hidden-c.rdie2e"]));
    let hooks = Hooks::install();
    let ms = send(&map);
    stats(&map, "first send");
    println!("1 hide: {ms} ms, hidden-a shown={} visible-a shown={}", shown("hidden-a.rdie2e"), shown("visible-a.rdie2e"));
    refresh();
    let (ever, now) = watch("hidden-a.rdie2e", 4000);
    stats(&map, "F5");
    println!("2 F5: hidden-a seen during refresh={ever} shown after={now} items={}", desk::positions().len());
    let b = file("hidden-b.rdie2e");
    let (ever, now) = watch("hidden-b.rdie2e", 3000);
    stats(&map, "new file");
    println!("3 new pinned file hidden-b: seen={ever} shown after={now}");
    fs::write(&a, b"probe changed").unwrap();
    let (ever, now) = watch("hidden-a.rdie2e", 2500);
    println!("4 modified hidden-a: seen={ever} shown after={now}");
    let c = d.join("hidden-c.rdie2e");
    fs::rename(&b, &c).unwrap();
    let (ever, now) = watch("hidden-c.rdie2e", 2500);
    println!("5 renamed hidden-b -> hidden-c: seen={ever} shown after={now}");
    map.write(&list(&["hidden-b.rdie2e", "hidden-c.rdie2e"]));
    send(&map);
    let (_, now) = watch("hidden-a.rdie2e", 1500);
    stats(&map, "unpin");
    println!(
        "6 unpin hidden-a: shown={now} pos={:?}",
        desk::positions().iter().find(|(p, _)| p.to_lowercase().ends_with("hidden-a.rdie2e")).map(|e| e.1)
    );
    map.write(&list(&["hidden-a.rdie2e", "hidden-c.rdie2e"]));
    send(&map);
    println!("7 pin again: hidden-a shown={}", shown("hidden-a.rdie2e"));
    let v = file("visible-b.rdie2e");
    let (_, now) = watch("visible-b.rdie2e", 2500);
    println!("8 new unpinned file visible-b shown={now}");
    map.write::<&str>(&[]);
    send(&map);
    let (_, now) = watch("hidden-a.rdie2e", 1500);
    stats(&map, "show all");
    println!("9 show all: hidden-a shown={now} hidden-c shown={}", shown("hidden-c.rdie2e"));
    drop(hooks);
    drop(map);
    let pid = explorer();
    let out = Command::new(std::env::current_exe().unwrap()).arg("--child").output().unwrap();
    print!("{}", String::from_utf8_lossy(&out.stdout));
    thread::sleep(Duration::from_millis(1000));
    println!("10 child died; explorer same pid={} hidden-a shown={}", explorer() == pid, shown("hidden-a.rdie2e"));
    refresh();
    let (_, now) = watch("hidden-a.rdie2e", 4000);
    println!("11 F5 after child death: hidden-a shown={now}, explorer same pid={}", explorer() == pid);
    if std::env::args().any(|a| a == "--restart-explorer") {
        let map = Map::create(std::process::id()).unwrap();
        map.write(&list(&["hidden-a.rdie2e"]));
        let hooks = Hooks::install();
        send(&map);
        println!("12 hidden again before restart: shown={}", shown("hidden-a.rdie2e"));
        let _ = Command::new("taskkill").args(["/f", "/im", "explorer.exe"]).output();
        thread::sleep(Duration::from_millis(1500));
        let _ = Command::new("explorer.exe").spawn();
        support::waited(30, || shown("visible-a.rdie2e"));
        thread::sleep(Duration::from_millis(3000));
        println!("13 explorer restarted (pid {}): hidden-a shown={}", explorer(), shown("hidden-a.rdie2e"));
        drop(hooks);
        let _hooks = Hooks::install();
        send(&map);
        stats(&map, "after restart");
        println!("14 re-hooked: hidden-a shown={}", shown("hidden-a.rdie2e"));
        map.write::<&str>(&[]);
        send(&map);
    }
    for p in [a, keep, c, v].iter().chain(&many) {
        let _ = fs::remove_file(p);
    }
    println!("done");
}
