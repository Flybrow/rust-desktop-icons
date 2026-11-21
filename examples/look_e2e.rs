mod support;

use serde_json::{Value, json};
use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    process::Child,
    thread,
    time::Duration,
};
use support::dump::{dump, keep};
use support::{app, chord, click, close, fences, grab, point, poll, quit, rect, tray, wait, windows, work};
use windows::Win32::{
    Foundation::HWND,
    UI::{
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::{VK_CONTROL, VK_ESCAPE, VK_F},
        WindowsAndMessaging::{GetCaretBlinkTime, WM_LBUTTONDBLCLK},
    },
};

const SQUARE: u64 = 1;
const ROUND: u64 = 2;
const ROLLED: u64 = 3;
const LIST: u64 = 4;
const ODD: u64 = 5;
const PHOTO: u64 = 6;
const ALL: [u64; 6] = [SQUARE, ROUND, ROLLED, LIST, ODD, PHOTO];

type Dump = (i32, i32, Vec<u32>);

fn picture() -> Option<PathBuf> {
    let mut todo = vec![PathBuf::from(r"C:\Windows\Web")];
    while let Some(d) = todo.pop() {
        for e in fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                todo.push(p);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("jpg")) {
                return Some(p);
            }
        }
    }
    None
}

fn alpha((w, _, px): &Dump, x: i32, y: i32) -> u32 {
    px.get((y * w + x) as usize).map_or(0, |p| p >> 24)
}

fn bands((w, _, px): &Dump, (x0, x1): (i32, i32), (y0, y1): (i32, i32), refx: i32) -> usize {
    let differs = |a: u32, b: u32| (0..4).any(|c| ((a >> (8 * c)) & 0xFF).abs_diff((b >> (8 * c)) & 0xFF) > 40);
    let px = |x: i32, y: i32| px.get((y * w + x) as usize).copied().unwrap_or(0);
    let rows: Vec<bool> = (y0..y1).map(|y| (x0..x1).any(|x| differs(px(x, y), px(refx, y)))).collect();
    rows.iter().enumerate().filter(|&(i, r)| *r && (i == 0 || !rows[i - 1])).count()
}

fn colors((w, _, px): &Dump, (x0, y0, x1, y1): (i32, i32, i32, i32)) -> usize {
    (y0..y1).flat_map(|y| (x0..x1).map(move |x| (y * w + x) as usize)).filter_map(|i| px.get(i)).collect::<HashSet<_>>().len()
}

fn bar(d: &Dump, t: i32, (x0, x1): (i32, i32)) -> Vec<u32> {
    (0..t).flat_map(|y| (x0..x1).map(move |x| (y * d.0 + x) as usize)).filter_map(|i| d.2.get(i).copied()).collect()
}

fn brightness(h: HWND) -> u32 {
    let (_, _, px) = grab(&rect(h));
    let n = (px.len() / 4).max(1) as u64;
    (px.chunks(4).map(|c| (c[0] as u64 + c[1] as u64 + c[2] as u64) / 3).sum::<u64>() / n) as u32
}

struct Session {
    child: Child,
    pid: u32,
}

fn start(root: &Path, shots: &Path, cfg: &Value) -> Session {
    let data = root.join("RustDesktopIcons");
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(cfg).unwrap()).unwrap();
    ALL.iter().for_each(|id| _ = fs::remove_file(shots.join(format!("{id}.bin"))));
    let child = app(root, &[("RDI_DUMP", shots.to_str().unwrap())], &[]);
    let pid = child.id();
    wait(10, || fences(pid).len() == ALL.len() && ALL.iter().all(|id| dump(shots, *id).is_some()));
    thread::sleep(Duration::from_millis(2500));
    Session { child, pid }
}

fn main() {
    let w = work();
    let root = env::temp_dir().join(format!("rdi-look-{}", std::process::id()));
    let (data, shots) = (root.join("RustDesktopIcons"), root.join("dumps"));
    let dir = |id: u64| data.join("fences").join(id.to_string());
    ALL.iter().for_each(|id| fs::create_dir_all(dir(*id)).unwrap());
    fs::create_dir_all(&shots).unwrap();
    for id in [SQUARE, ROUND, ROLLED, LIST] {
        ["a.txt", "b.txt", "c.txt"].iter().for_each(|f| fs::write(dir(id).join(f), "x").unwrap());
    }
    fs::write(dir(ODD).join("a.txt"), "x").unwrap();
    let pic = picture();
    if let Some(p) = &pic {
        fs::copy(p, dir(PHOTO).join("photo.jpg")).unwrap();
    }
    let at = |id: u64| {
        let i = id as i32 - 1;
        (w.left + 10 + (i % 3) * 440, w.top + 40 + (i / 3) * 270)
    };
    let fence = |id: u64, extra: Value| {
        let (x, y) = at(id);
        let mut f = json!({"id": id, "x": x, "y": y, "w": 420, "h": 220, "auto_height": false, "custom": [],
            "tabs": [{"id": id, "title": format!("Look {id}")}]});
        f.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        f
    };
    let config = |theme: &str, privacy: Value| {
        json!({"lang": "en", "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "peek_key": null, "search_key": null,
            "theme": theme, "privacy": privacy, "fences": [
                fence(SQUARE, json!({"corner": "Square", "custom": ["Corner"]})),
                fence(ROUND, json!({"corner": "Large", "custom": ["Corner"]})),
                fence(ROLLED, json!({"corner": "Large", "rolled": true, "custom": ["Corner"]})),
                fence(LIST, json!({"list": true, "custom": ["List"]})),
                fence(ODD, json!({"icon": 40, "custom": ["Icon"]})),
                fence(PHOTO, json!({}))]})
    };
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| {
        support::verdict(ok, what, &mut fails);
    };
    let read = |id: u64| poll(3, || dump(&shots, id)).unwrap_or_default();
    let hwnd = |pid: u32, id: u64| {
        let (x, y) = at(id);
        windows(pid, "RustDesktopIcons.Fence").into_iter().find(|h| (rect(*h).left - x).abs() < 40 && (rect(*h).top - y).abs() < 40)
    };
    let away = || point((w.right - 20, w.bottom - 20));
    let settings = |pid: u32| {
        tray(pid, WM_LBUTTONDBLCLK);
        let h = poll(3, || windows(pid, "RustDesktopIcons.Settings").first().copied());
        thread::sleep(Duration::from_millis(800));
        let lum = h.map(brightness);
        h.into_iter().for_each(close);
        lum
    };

    away();
    let mut s = start(&root, &shots, &config("Dark", json!({"on": false})));
    let Some(h) = hwnd(s.pid, ROUND) else {
        quit(&mut s.child);
        println!("fences not shown");
        std::process::exit(1);
    };
    let k = unsafe { GetDpiForWindow(h) }.max(96) as i32;
    let sc = |v: i32| v * k / 96;
    let t = sc(34);
    let body = |d: &Dump| (sc(8), t + sc(4), d.0 - sc(20), d.1 - sc(8));

    let sq = read(SQUARE);
    check(alpha(&sq, 0, 0) > 0, "Square corners: the corner pixel is drawn");
    let rd = read(ROUND);
    check(alpha(&rd, 0, 0) == 0 && alpha(&rd, rd.0 / 2, rd.1 / 2) > 0, "Large corners: the corner is cut");
    let rolled = read(ROLLED);
    let (rw, rh) = (rolled.0, rolled.1);
    keep(&shots, ROLLED, "look-rolled");
    check(
        rh > 0 && rh <= t + sc(2) && [(0, 0), (rw - 1, 0), (0, rh - 1), (rw - 1, rh - 1)].iter().all(|&(x, y)| alpha(&rolled, x, y) == 0),
        "a rolled fence keeps rounded corners on all four sides",
    );
    let lines = |d: &Dump| {
        let (x0, y0, x1, y1) = body(d);
        bands(d, (x0, x1), (y0, y1), d.0 - sc(10))
    };
    let (list, icons) = (lines(&read(LIST)), lines(&sq));
    keep(&shots, LIST, "look-list");
    check(list == 3 && icons <= 2, &format!("Show as a list puts one item per row ({list} rows, {icons} in icon view)"));
    check(lines(&read(ODD)) >= 1, "an icon at a size Windows does not provide (40 px) is drawn");
    let photo = read(PHOTO);
    let plain = colors(&photo, body(&photo));

    let before = bar(&rd, t, (0, rd.0));
    let r = rect(h);
    click((r.right - sc(30), r.bottom - sc(30)));
    chord(&[VK_CONTROL, VK_F]);
    let opened = wait(3, || bar(&read(ROUND), t, (0, rd.0)) != before);
    let field = read(ROUND);
    keep(&shots, ROUND, "look-filter");
    let end = (field.0 - t, field.0 - sc(6));
    check(opened && bar(&field, t, end) != bar(&rd, t, end), "Ctrl+F shows a search field with a close mark in the bar");
    let blink = unsafe { GetCaretBlinkTime() }.clamp(200, 1200) as u64;
    let caret = (sc(26), sc(40));
    let samples: HashSet<Vec<u32>> = (0..6)
        .map(|_| {
            thread::sleep(Duration::from_millis(blink / 2));
            bar(&read(ROUND), t, caret)
        })
        .collect();
    check(samples.len() >= 2, "the caret blinks in the search field");
    chord(&[VK_ESCAPE]);
    check(wait(3, || bar(&read(ROUND), t, (0, rd.0)) == before), "Escape removes the search field");
    away();
    let dark = settings(s.pid);
    check(dark.is_some_and(|l| l < 110), &format!("Dark theme: the settings window is dark ({dark:?})"));
    quit(&mut s.child);

    let mut s = start(&root, &shots, &config("Light", json!({"on": true, "names": true, "icons": true, "reveal": "Hover"})));
    let masked = colors(&read(PHOTO), body(&photo));
    keep(&shots, PHOTO, "look-private");
    check(
        pic.is_none() || (plain > 60 && masked * 3 < plain),
        &format!("Privacy mode masks icons and names ({masked} colors, {plain} without it)"),
    );
    if let Some(p) = hwnd(s.pid, PHOTO) {
        let r = rect(p);
        point((r.right - sc(20), r.bottom - sc(20)));
        let shown = wait(3, || colors(&read(PHOTO), body(&photo)) * 3 > plain * 2);
        check(pic.is_none() || shown, "Reveal on hover: the mouse over the fence shows them");
        away();
        check(wait(3, || colors(&read(PHOTO), body(&photo)) * 3 < plain), "they are masked again when the mouse leaves");
    }
    let light = settings(s.pid);
    check(light.is_some_and(|l| l > 170), &format!("Light theme: the settings window is light ({light:?})"));
    quit(&mut s.child);

    let mut s = start(&root, &shots, &config("Dark", json!({"on": true, "names": true, "icons": true, "reveal": "Never"})));
    if let Some(p) = hwnd(s.pid, PHOTO) {
        let r = rect(p);
        point((r.right - sc(20), r.bottom - sc(20)));
        thread::sleep(Duration::from_millis(1200));
        check(pic.is_none() || colors(&read(PHOTO), body(&photo)) * 3 < plain, "Reveal never: hovering keeps them masked");
        away();
    }
    quit(&mut s.child);

    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("all look checks passed");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
