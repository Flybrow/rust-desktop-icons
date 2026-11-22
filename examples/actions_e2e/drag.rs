use crate::support::{self, click, drag, menu, rect, right_click, saved, wait};
use crate::{Run, fence_menu, first_item};
use serde_json::{Value, json};
use std::{fs, thread, time::Duration};
use windows::Win32::{Foundation::HWND, UI::HiDpi::GetDpiForWindow};

pub fn run(t: &mut Run, main: HWND, other: HWND) -> bool {
    if let (Some(h), Some(del)) = (t.hwnd("Drag"), t.hwnd("Del")) {
        let (k, at) = (unsafe { GetDpiForWindow(h) } as i32, first_item(h));
        let cell = 104 * k / 96;
        drag((at.0 + cell, at.1), (at.0 - cell / 3, at.1));
        t.check(
            t.until(|_| t.fence(6)["tabs"][0]["order"].get(0) == Some(&json!("d2.txt"))),
            "dragging an item before another reorders the tab",
        );
        let r = rect(del);
        drag((at.0, at.1 + cell), ((r.left + r.right) / 2, (r.top + r.bottom) / 2));
        t.check(
            wait(5, || t.dir(5).join("d3.txt").exists() && !t.dir(6).join("d3.txt").exists()),
            "dragging an item onto another fence moves it there",
        );
        thread::sleep(Duration::from_millis(500));
        let b = rect(h);
        drag((b.right - 15, b.bottom - 15), (b.left + 8, b.top + 44 * k / 96));
        thread::sleep(Duration::from_millis(300));
        right_click(at);
        menu::open();
        menu::pick(&["Move to", "Other"]);
        let both = || ["d1.txt", "d2.txt"].iter().all(|n| t.dir(2).join(n).exists() && !t.dir(6).join(n).exists());
        t.check(wait(5, both), "a rubber-band selection moves together (Move to)");
    }
    fence_menu(other, &["Tabs", "New tab"]);
    let two = t.until(|_| t.fence(2)["tabs"].as_array().is_some_and(|a| a.len() == 2) && t.fence(2)["tab"] == 1);
    let r = rect(other);
    thread::sleep(Duration::from_millis(400));
    click((r.left + (r.right - r.left) / 4, r.top + 12));
    thread::sleep(Duration::from_millis(400));
    duplicate(t, main);
    two
}

fn duplicate(t: &mut Run, main: HWND) {
    let n = support::fences(t.pid).len();
    fence_menu(main, &["Duplicate"]);
    let copies = |c: &Value| c["fences"].as_array().map_or(0, |a| a.iter().filter(|f| f["tabs"][0]["title"] == "Renamed").count());
    t.check(wait(3, || support::fences(t.pid).len() == n + 1) && t.until(|c| copies(c) == 2), "Duplicate adds a copy with the same tabs");
    let copy = saved(&t.data)["fences"]
        .as_array()
        .and_then(|a| a.iter().rev().find(|f| f["tabs"][0]["title"] == "Renamed").cloned())
        .unwrap_or_default();
    let tab = copy["tabs"][0]["id"].as_u64().unwrap_or(0);
    let empty = fs::read_dir(t.dir(tab)).map(|d| d.count() == 0).unwrap_or(true);
    t.check(copy["id"] != json!(1) && copy["x"] != t.fence(1)["x"] && empty, "the copy sits next to the original, without its files");
    let n = |f: &Value, k: &str| f[k].as_i64().unwrap_or(0);
    let apart = |a: &Value, b: &Value| {
        n(a, "x") + n(a, "w") <= n(b, "x")
            || n(b, "x") + n(b, "w") <= n(a, "x")
            || n(a, "y") + n(a, "h") <= n(b, "y")
            || n(b, "y") + n(b, "h") <= n(a, "y")
    };
    let others = saved(&t.data)["fences"].as_array().cloned().unwrap_or_default();
    t.check(others.iter().filter(|f| f["id"] != copy["id"]).all(|f| apart(f, &copy)), "the copy does not cover another fence");
}
