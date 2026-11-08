mod support;

use serde_json::{Value, json};
use std::{
    env, fs,
    time::{Duration, Instant},
};
use support::{app, ctrl_wheel, fences, point, quit, usage::usage, waited, work};
use windows::Win32::{
    Foundation::{CloseHandle, FILETIME},
    System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
};

const BIG: usize = 3000;
const SMALL: u64 = 40;

fn cpu_ms(pid: u32) -> u64 {
    let Ok(h) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else { return 0 };
    let (mut a, mut b, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
    let _ = unsafe { GetProcessTimes(h, &mut a, &mut b, &mut k, &mut u) };
    let _ = unsafe { CloseHandle(h) };
    let t = |f: FILETIME| ((f.dwHighDateTime as u64) << 32 | f.dwLowDateTime as u64) / 10_000;
    t(k) + t(u)
}

fn main() {
    let w = work();
    let root = env::temp_dir().join(format!("rdi-load-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    let dir = |id: u64| data.join("fences").join(id.to_string());
    fs::create_dir_all(dir(1)).unwrap();
    for i in 0..BIG {
        fs::write(dir(1).join(format!("document {i:04}.txt")), "x").unwrap();
    }
    let mut list: Vec<Value> = vec![
        json!({"id": 1, "x": w.left + 20, "y": w.top + 20, "w": 600, "h": 500, "auto_height": false, "tabs": [{"id": 1, "title": "Big"}]}),
    ];
    for id in 2..2 + SMALL {
        fs::create_dir_all(dir(id)).unwrap();
        for i in 0..20 {
            fs::write(dir(id).join(format!("f{i}.txt")), "x").unwrap();
        }
        let (k, cols) = ((id - 2) as i32, ((w.right - w.left - 660) / 125).max(1));
        let (x, y) = (w.left + 660 + (k % cols) * 125, w.top + 20 + (k / cols) * 95);
        list.push(
            json!({"id": id, "x": x, "y": y, "w": 120, "h": 90, "auto_height": false, "tabs": [{"id": id, "title": format!("S{id}")}]}),
        );
    }
    let cfg = json!({"lang": "en", "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "fences": list});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| support::verdict(ok, what, &mut fails);

    let start = Instant::now();
    let mut child = app(&root, &[("RDI_TOP", "1")], &[]);
    let pid = child.id();
    let all = 1 + SMALL as usize;
    let shown = waited(60, || fences(pid).len() == all);
    let ms = start.elapsed().as_millis();
    println!("      {all} fences ({BIG} + {} files) drawn after {ms} ms (debug build)", SMALL * 20);
    check(shown.is_some() && ms < 30_000, "41 fences with 3,800 files are all drawn within 30 s (debug build)");
    std::thread::sleep(Duration::from_secs(5));
    let u = usage(pid).unwrap_or_default();
    println!("      private {:.1} MB, GDI {}, USER {}, handles {}", u.private_mb, u.gdi, u.user, u.handles);
    check(u.private_mb < 400.0, &format!("memory stays reasonable ({:.0} MB)", u.private_mb));
    check(u.gdi < 2000 && u.user < 2000, "GDI and USER objects stay far below the 10,000 limit");
    point((w.right - 20, w.bottom - 20));
    std::thread::sleep(Duration::from_secs(2));
    let before = cpu_ms(pid);
    std::thread::sleep(Duration::from_secs(10));
    let idle = cpu_ms(pid) - before;
    println!("      CPU while idle: {idle} ms over 10 s");
    check(idle < 500, &format!("idle CPU stays near zero ({idle} ms in 10 s)"));
    point((w.left + 300, w.top + 300));
    let t = Instant::now();
    for _ in 0..10 {
        support::wheel(false);
    }
    ctrl_wheel(true);
    std::thread::sleep(Duration::from_millis(300));
    check(
        t.elapsed() < Duration::from_secs(10) && child.try_wait().ok().flatten().is_none(),
        "scrolling and zooming the big fence stays responsive",
    );
    quit(&mut child);
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("LOAD E2E OK");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
