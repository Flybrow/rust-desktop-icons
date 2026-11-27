mod support;

use serde_json::{Value, json};
use std::{
    env, fs, thread,
    time::{Duration, Instant},
};
use support::dump::{Dump, bands, bar, dump, edges, keep};
use support::{
    app, chord, double_click, drag, fence_cfg, fences, glide, hop, menu, point, poll, quit, rect, right_click, wait, windows, work,
};
use windows::Win32::UI::{
    HiDpi::GetDpiForWindow,
    Input::KeyboardAndMouse::{VK_CONTROL, VK_ESCAPE, VK_Z},
};

const DETAILS: u64 = 1;
const TILES: u64 = 2;
const CONTENT: u64 = 3;
const ICONS: u64 = 4;
const PORTAL: u64 = 5;
const ROLL: u64 = 7;
const FEW: u64 = 8;
const ALL: [u64; 6] = [DETAILS, TILES, CONTENT, ICONS, PORTAL, ROLL];

fn main() {
    let w = work();
    let root = env::temp_dir().join(format!("rdi-views-{}", std::process::id()));
    let (data, shots, docs) = (root.join("RustDesktopIcons"), root.join("dumps"), root.join("docs"));
    let dir = |id: u64| data.join("fences").join(id.to_string());
    [DETAILS, TILES, CONTENT, ICONS, 6, ROLL, FEW].iter().for_each(|id| fs::create_dir_all(dir(*id)).unwrap());
    fs::create_dir_all(&shots).unwrap();
    fs::create_dir_all(&docs).unwrap();
    for id in [DETAILS, TILES, CONTENT, ICONS] {
        for (f, n) in [("a.txt", 10), ("b.txt", 5000), ("c.txt", 300)] {
            fs::write(dir(id).join(f), "x".repeat(n)).unwrap();
        }
    }
    fs::write(docs.join("doc.txt"), "x").unwrap();
    (1..=4).for_each(|i| fs::write(dir(ROLL).join(format!("a{i}.txt")), "x").unwrap());
    fs::create_dir_all(dir(ROLL).join("z-sub")).unwrap();
    fs::write(dir(ROLL).join("z-sub").join("inside.txt"), "x").unwrap();
    fs::write(dir(FEW).join("one.txt"), "x").unwrap();

    let at = |id: u64| {
        let i = id.min(6) as i32 - 1;
        (w.left + support::scale(10 + (i % 3) * 440), w.top + support::scale(40 + (i / 3) * 280))
    };
    let tab = |id: u64, view: &str| json!({"id": id, "title": format!("View {id}"), "view": view});
    let fence = |id: u64, tabs: Value, extra: Value| {
        let (x, y) = at(id);
        let mut f = json!({"id": id, "x": x, "y": y, "w": support::scale(420), "h": support::scale(230), "auto_height": false, "custom": [], "tabs": tabs});
        f.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        f
    };
    let cfg = json!({"lang": "en", "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"), "peek_key": null,
        "search_key": null, "fences": [
            fence(DETAILS, json!([tab(DETAILS, "Details")]), json!({})),
            fence(TILES, json!([tab(TILES, "Tiles")]), json!({})),
            fence(CONTENT, json!([tab(CONTENT, "Content")]), json!({})),
            fence(ICONS, json!([{"id": ICONS, "title": "Icons"}]), json!({})),
            fence(PORTAL, json!([{"id": PORTAL, "title": "Docs", "portal": docs}, {"id": 6, "title": "Other"}]), json!({})),
            fence(ROLL, json!([{"id": ROLL, "title": "Rolled"}, {"id": FEW, "title": "Few"}]), json!({"rolled": true, "auto_height": true, "list": true, "custom": ["List"]}))]});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    point((w.right - 20, w.bottom - 20));
    let mut child = app(&root, &[("RDI_DUMP", shots.to_str().unwrap())], &[]);
    let pid = child.id();
    let shown = wait(10, || fences(pid).len() == ALL.len() && ALL.iter().all(|id| dump(&shots, *id).is_some()));
    thread::sleep(Duration::from_millis(2500));
    let hwnd = |id: u64| {
        let (x, y) = at(id);
        windows(pid, "RustDesktopIcons.Fence").into_iter().find(|h| (rect(*h).left - x).abs() < 40 && (rect(*h).top - y).abs() < 40)
    };
    let Some(h) = hwnd(DETAILS).filter(|_| shown) else {
        quit(&mut child);
        println!("fences not shown");
        std::process::exit(1);
    };
    let mut fails = vec![];
    let mut check = |ok: bool, what: &str| support::verdict(ok, what, &mut fails);
    let read = |id: u64| poll(3, || dump(&shots, id)).unwrap_or_default();
    let k = unsafe { GetDpiForWindow(h) }.max(96) as i32;
    let sc = |v: i32| v * k / 96;
    let t = sc(support::bar_h());
    let tab0 = |id: u64| fence_cfg(&data, id)["tabs"][0].clone();
    let text = |d: &Dump, (x0, x1): (i32, i32), (y0, y1): (i32, i32)| bands(d, (x0, x1), (y0, y1));

    let d = read(DETAILS);
    keep(&shots, DETAILS, "views-details");
    let rows = text(&d, (sc(8), d.0 - sc(12)), (t, d.1 - sc(8)));
    check(rows >= 4, &format!("Details: a header and one row per item ({rows} text rows)"));
    let size = edges(&d, (d.0 - sc(70), t + sc(30), d.0 - sc(10), t + sc(110)));
    check(size > 8, &format!("Details: the Size column shows text ({size} colors)"));
    let tl = read(TILES);
    keep(&shots, TILES, "views-tiles");
    let lines = text(&tl, (sc(60), sc(250)), (t, t + sc(70)));
    check(lines >= 3, &format!("Tiles: a tile shows the name, the type and the size ({lines} lines)"));
    let ct = read(CONTENT);
    keep(&shots, CONTENT, "views-content");
    let right = edges(&ct, (ct.0 * 11 / 20 + sc(12), t + sc(4), ct.0 - sc(12), t + sc(52)));
    let left = text(&ct, (sc(48), ct.0 * 11 / 20), (t + sc(4), t + sc(52)));
    check(
        right > 8 && left >= 2,
        &format!("Content: name and type on the left, date and size on the right ({left} lines, {right} colors)"),
    );

    let r = rect(h);
    let head = r.top + t + sc(4) + sc(13);
    support::click((r.right - sc(38), head));
    check(wait(3, || tab0(DETAILS)["sort"] == "Size" && tab0(DETAILS)["desc"].is_null()), "a click on the Size header sorts by size");
    support::click((r.right - sc(38), head));
    check(wait(3, || tab0(DETAILS)["desc"] == true), "a second click reverses the order");
    let edge = r.right - sc(76 + 120);
    drag((edge, head), (edge - sc(30), head));
    let width = |i: usize| tab0(DETAILS)["columns"][i]["w"].as_i64().unwrap_or(0);
    let moved = wait(3, || (width(0) - 100).abs() <= 3 && (width(1) - 150).abs() <= 3);
    check(moved, &format!("dragging a header edge moves the border between its two columns ({}, {})", width(0), width(1)));
    chord(&[VK_CONTROL, VK_Z]);
    check(wait(3, || tab0(DETAILS)["columns"].is_null()), "Ctrl+Z undoes a column resize");
    right_click((r.left + sc(60), head));
    let created = support::poll(3, || menu::checked("Date created")) == Some(false) && menu::pick(&["Date created"]);
    let cols = || tab0(DETAILS)["columns"].as_array().map_or(vec![], |a| a.iter().map(|c| c["col"].to_string()).collect::<Vec<_>>());
    check(created && wait(3, || cols().iter().any(|c| c.contains("Created"))), "the header menu adds the Date created column");

    if let Some(i) = hwnd(ICONS) {
        let r = rect(i);
        right_click((r.right - sc(30), r.bottom - sc(30)));
        let picked = menu::pick(&["View", "Details"]);
        check(picked && wait(3, || tab0(ICONS)["view"] == "Details"), "View ▸ Details switches the tab to the details view");
        check(
            wait(3, || text(&read(ICONS), (sc(8), rect(i).right - rect(i).left - sc(12)), (t, sc(150))) >= 4),
            "the switched tab shows its header",
        );
    }

    if let Some(p) = hwnd(PORTAL) {
        let r = rect(p);
        let len = r.right - r.left;
        let before = read(PORTAL);
        point((r.left + sc(60), r.bottom - sc(40)));
        let seg = (len / 2 - t, len / 2);
        let hovered = wait(3, || bar(&read(PORTAL), t, seg) != bar(&before, t, seg));
        let now = read(PORTAL);
        keep(&shots, PORTAL, "views-loupe");
        let far = (len - t, len);
        check(hovered && bar(&now, t, far) == bar(&before, t, far), "the magnifier sits at the end of the active tab, not of the bar");
        support::click((r.left + len / 2 - t / 2, r.top + t / 2));
        check(wait(3, || bar(&read(PORTAL), t, (0, len)) != bar(&now, t, (0, len))), "a click on it opens the filter of that tab");
        chord(&[VK_ESCAPE]);
        point((w.right - 20, w.bottom - 20));
    }

    if let Some(rl) = hwnd(ROLL) {
        let r = rect(rl);
        let height = || rect(rl).bottom - rect(rl).top;
        glide((w.right - 20, w.bottom - 20), (r.left + sc(100), r.top + t / 2));
        let open = wait(3, || height() > sc(220));
        thread::sleep(Duration::from_millis(800));
        let full = height();
        let row = sc(32 + 12);
        let target = (r.left + sc(80), r.top + t + sc(4) + 4 * row + row / 2);
        glide((r.left + sc(100), r.top + t / 2), target);
        double_click(target);
        let body = |d: &Dump| text(d, (sc(8), d.0 - sc(12)), (t + sc(2), d.1 - sc(6)));
        let inside = wait(3, || body(&read(ROLL)) == 1);
        thread::sleep(Duration::from_millis(1500));
        keep(&shots, ROLL, "views-rolled");
        let (now, rows) = (height(), body(&read(ROLL)));
        let what = format!("a rolled fence keeps its size and folder when a smaller folder opens ({full} -> {now}, {rows} row)");
        check(open && inside && now == full && rows == 1, &what);
        point((w.right - 20, w.bottom - 20));
        check(wait(3, || height() <= t + sc(2)), "it rolls up once the mouse moves away");
        glide((w.right - 20, w.bottom - 20), (r.left + sc(100), r.top + t / 2));
        let open = wait(3, || height() > sc(220));
        thread::sleep(Duration::from_millis(800));
        let full = height();
        glide((r.left + sc(100), r.top + t / 2), (r.left + sc(320), r.top + t / 2));
        let shrunk = wait(3, || height() < full - row);
        glide((r.left + sc(320), r.top + t / 2), (r.left + sc(100), r.top + t / 2));
        let grown = wait(3, || height() == full);
        check(
            open && shrunk && grown,
            &format!("an open rolled fence takes the height of each tab it switches to ({full} -> {})", height()),
        );
        glide((r.left + sc(100), r.top + t / 2), (r.left + sc(100), r.top + t / 2 + 1));
        let full = height();
        hop(&[(r.left + sc(320), r.top + t / 2)], 0);
        let (start, mut seen) = (Instant::now(), vec![]);
        while start.elapsed() < Duration::from_millis(900) {
            seen.push(height());
            thread::sleep(Duration::from_millis(8));
        }
        let small = height();
        let between = seen.iter().filter(|v| **v < full && **v > small).count();
        check(small < full && between >= 3, &format!("switching tabs glides the height ({full} -> {small}, {between} steps between)"));
        point((w.right - 20, w.bottom - 20));
        wait(3, || height() <= t + sc(2));
    }

    quit(&mut child);
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("all views checks passed");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
