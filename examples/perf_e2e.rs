mod support;

use serde_json::json;
use std::{env, fs, thread, time::Duration};
use support::{app, glide, point, quit, rect, wait, wheel, windows, work};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

const ROLL: u64 = 1;
const BIG: u64 = 2;
const TABS: u64 = 3;

fn now() -> f64 {
    let (mut t, mut f) = (0i64, 0i64);
    unsafe {
        let _ = QueryPerformanceCounter(&mut t);
        let _ = QueryPerformanceFrequency(&mut f);
    }
    t as f64 * 1000. / f as f64
}

fn pct(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() {
        return 0.;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((v.len() - 1) as f64 * p).round() as usize]
}

fn stats(name: &str, v: &[f64]) -> String {
    let mut v = v.to_vec();
    let avg = if v.is_empty() { 0. } else { v.iter().sum::<f64>() / v.len() as f64 };
    format!("{name}: n={} avg={avg:.1} p50={:.1} p95={:.1} max={:.1}", v.len(), pct(&mut v, 0.5), pct(&mut v, 0.95), pct(&mut v, 1.))
}

fn main() {
    let w = work();
    let root = env::temp_dir().join(format!("rdi-perf-{}", std::process::id()));
    let data = root.join("RustDesktopIcons");
    let dir = |id: u64| data.join("fences").join(id.to_string());
    for (id, n) in [(ROLL, 40), (BIG, 400), (TABS, 30), (4, 12), (5, 60)] {
        fs::create_dir_all(dir(id)).unwrap();
        (0..n).for_each(|i| fs::write(dir(id).join(format!("file {i:03} report.txt")), "x").unwrap());
    }
    let fence = |id: u64, x: i32, extra: serde_json::Value| {
        let mut f = json!({"id": id, "x": w.left + x, "y": w.top + if id == TABS { 480 } else { 40 }, "w": 600, "h": 420, "custom": [], "tabs": [{"id": id, "title": format!("Perf {id}")}]});
        f.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        f
    };
    let cfg = json!({"lang": "en", "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "peek_key": null,
        "search_key": null, "fences": [fence(ROLL, 20, json!({"rolled": true})), fence(BIG, 700, json!({"auto_height": false})),
            fence(TABS, 20, json!({"rolled": true, "tabs": [{"id": 3, "title": "Tab A"}, {"id": 4, "title": "Tab B"}, {"id": 5, "title": "Tab C"}]}))]});
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let away = (w.right - 20, w.bottom - 20);
    point(away);
    let mut child = app(&root, &[("RDI_PERF", "1")], &[]);
    let pid = child.id();
    if !wait(15, || windows(pid, "RustDesktopIcons.Fence").len() == 3) {
        quit(&mut child);
        println!("fences not shown");
        std::process::exit(1);
    }
    thread::sleep(Duration::from_millis(3000));
    let all = windows(pid, "RustDesktopIcons.Fence");
    let find = |x: i32, y: i32| {
        all.iter().copied().find(|h| (rect(*h).left - (w.left + x)).abs() < 40 && (rect(*h).top - (w.top + y)).abs() < 300)
    };
    let (Some(roll), Some(big), Some(tabs)) = (find(20, 40), find(700, 40), all.iter().copied().find(|h| rect(*h).top > w.top + 300))
    else {
        quit(&mut child);
        println!("fences not found");
        std::process::exit(1);
    };
    let r = rect(roll);
    let bar = (r.left + 300, r.top + 15);
    let mut phases = vec![];
    let mut phase = |name: &str, f: &mut dyn FnMut()| {
        let t0 = now();
        f();
        phases.push((name.to_string(), t0, now()));
    };
    phase("unroll (hover the bar)", &mut || {
        for _ in 0..5 {
            glide(away, bar);
            thread::sleep(Duration::from_millis(1200));
            glide(bar, away);
            thread::sleep(Duration::from_millis(1200));
        }
    });
    phase("unroll while moving over the icons", &mut || {
        for _ in 0..5 {
            glide(away, bar);
            glide(bar, (r.left + 300, r.top + 300));
            thread::sleep(Duration::from_millis(1200));
            glide((r.left + 300, r.top + 300), away);
            thread::sleep(Duration::from_millis(1200));
        }
    });
    let tb = rect(tabs);
    let seg = |i: i32| (tb.left + 100 + i * 200, (tb.top + tb.bottom) / 2);
    phase("rolled fence with tabs: unroll, switch tabs on the bar, roll up", &mut || {
        for _ in 0..5 {
            glide(away, seg(0));
            thread::sleep(Duration::from_millis(800));
            glide(seg(0), seg(1));
            thread::sleep(Duration::from_millis(700));
            glide(seg(1), seg(2));
            thread::sleep(Duration::from_millis(700));
            glide(seg(2), seg(0));
            thread::sleep(Duration::from_millis(700));
            glide(seg(0), away);
            thread::sleep(Duration::from_millis(1200));
        }
    });
    let b = rect(big);
    phase("wheel scroll (400 icons)", &mut || {
        point((b.left + 300, b.top + 200));
        thread::sleep(Duration::from_millis(600));
        for k in 0..40 {
            wheel(k >= 20);
            thread::sleep(Duration::from_millis(60));
        }
    });
    phase("hover over icons (400 icons)", &mut || {
        for k in 0..4 {
            let y = b.top + 80 + k * 80;
            glide((b.left + 20, y), (b.right - 20, y));
        }
    });
    point(away);
    thread::sleep(Duration::from_millis(500));
    quit(&mut child);
    thread::sleep(Duration::from_millis(500));
    let log = fs::read_to_string(data.join("perf.log")).unwrap_or_default();
    let lines: Vec<(f64, isize, String, f64)> = log
        .lines()
        .filter_map(|l| {
            let mut p = l.split(' ');
            Some((p.next()?.parse().ok()?, p.next()?.parse().ok()?, p.next()?.to_string(), p.next()?.parse().ok()?))
        })
        .collect();
    println!("{} perf lines, pacer period {:?} ms", lines.len(), lines.iter().find(|l| l.2 == "period").map(|l| l.3));
    for (name, t0, t1) in &phases {
        println!("== {name} ({:.0} ms)", t1 - t0);
        for (label, h) in [("rolled", roll.0 as isize), ("big", big.0 as isize), ("tabs", tabs.0 as isize)] {
            let ev: Vec<&(f64, isize, String, f64)> = lines.iter().filter(|l| l.0 >= *t0 && l.0 <= *t1 && l.1 == h).collect();
            if ev.is_empty() {
                continue;
            }
            let of = |k: &str| ev.iter().filter(|e| e.2 == k).map(|e| e.3).collect::<Vec<f64>>();
            let ticks: Vec<f64> = ev.iter().filter(|e| e.2.starts_with("tick.cached") || e.2 == "tick.miss").map(|e| e.0).collect();
            let gaps: Vec<f64> = ticks.windows(2).map(|p| p[1] - p[0]).filter(|g| *g < 100.).collect();
            println!(
                "  [{label}] starts={} ticks={} misses={} drops={}",
                of("start").len(),
                ticks.len(),
                of("tick.miss").len(),
                of("render.drop").len()
            );
            println!("  [{label}] {}", stats("tick interval ms", &gaps));
            println!("  [{label}] {}", stats("tick draw ms", &of("tick.draw")));
            println!("  [{label}] {}", stats("tick present ms", &of("tick.present")));
            println!("  [{label}] {}", stats("render ms", &of("render")));
            println!("  [{label}] {}", stats("prepare ms", &of("prepare")));
        }
    }
    let _ = fs::remove_dir_all(&root);
}
