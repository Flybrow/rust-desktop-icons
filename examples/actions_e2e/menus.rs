use crate::support::{self, chord, close, ctrl_wheel, drag, explorer, menu, poll, rect, right_click, saved, type_text, wait, windows};
use crate::{Run, empty_spot, fence_menu, has};
use serde_json::{Value, json};
use std::{thread, time::Duration};
use windows::Win32::{Foundation::HWND, UI::Input::KeyboardAndMouse::*};

pub fn run(t: &mut Run, main: HWND, other: HWND) {
    fence_menu(main, &["Rename"]);
    thread::sleep(Duration::from_millis(300));
    chord(&[VK_CONTROL, VK_A]);
    type_text("Renamed");
    chord(&[VK_RETURN]);
    t.check(t.until(|c| c["fences"][0]["tabs"][0]["title"] == "Renamed"), "Rename changes the fence title");

    fence_menu(main, &["Sort by", "Size"]);
    t.check(t.until(|_| t.fence(1)["tabs"][0]["sort"] == "Size"), "Sort by → Size is saved");
    fence_menu(main, &["Sorting rules", "Images"]);
    t.check(t.until(|_| has(&t.fence(1)["tabs"][0]["kinds"], "Images")), "Sorting rules → Images adds the rule");

    look(t, main);
    lock_and_group(t, main);

    for (label, field) in [("Roll up / down", "rolled"), ("Vertical title", "vertical")] {
        fence_menu(main, &["Behavior", label]);
        let on = t.until(|_| t.fence(1)[field] == true);
        thread::sleep(Duration::from_millis(600));
        let h = t.hwnd("Renamed").unwrap_or(main);
        fence_menu(h, &["Behavior", label]);
        t.check(on && t.until(|_| t.fence(1)[field] == false), &format!("{label} toggles"));
        thread::sleep(Duration::from_millis(600));
    }

    fence_menu(main, &["Tabs", "New tab"]);
    t.check(t.until(|_| t.fence(1)["tabs"].as_array().is_some_and(|a| a.len() == 2)), "New tab adds a tab");
    thread::sleep(Duration::from_millis(400));
    fence_menu(main, &["Tabs", "Delete tab"]);
    let asked = support::answer(t.pid, &[VK_RETURN]);
    t.check(asked && t.until(|_| t.fence(1)["tabs"].as_array().is_some_and(|a| a.len() == 1)), "Delete tab asks, then removes the tab");
    fence_menu(main, &["Tabs", "New portal tab"]);
    let picker = support::answer(t.pid, &[VK_ESCAPE]);
    t.check(
        picker && t.fence(1)["tabs"].as_array().is_some_and(|a| a.len() == 1),
        "New portal tab opens a folder picker; cancel adds nothing",
    );

    let before = t.fence(1)["icon"].as_i64();
    support::point(empty_spot(main));
    ctrl_wheel(true);
    t.check(t.until(|_| t.fence(1)["icon"].as_i64() > before), "Ctrl + mouse wheel enlarges the icons of that fence");

    fence_menu(main, &["Show the fence in File Explorer"]);
    let opened = poll(5, || explorer("1"));
    t.check(opened.is_some(), "Open folder shows the fence folder in Explorer");
    opened.into_iter().for_each(close);
    wait(5, || explorer("1").is_none());
    thread::sleep(Duration::from_millis(400));

    let n = support::fences(t.pid).len();
    fence_menu(main, &["New fence"]);
    t.check(wait(3, || support::fences(t.pid).len() == n + 1), "New fence creates a fence");
    thread::sleep(Duration::from_millis(600));
    let fresh = windows(t.pid, "RustDesktopIcons.Fence").into_iter().find(|h| ![main, other].contains(h) && t.hwnd("Fence") == Some(*h));
    if let Some(h) = fresh {
        fence_menu(h, &["Delete fence"]);
        let asked = support::answer(t.pid, &[VK_RETURN]);
        t.check(asked && wait(3, || support::fences(t.pid).len() == n), "Delete fence asks, then removes the fence");
    } else {
        t.check(false, "the new fence is found");
    }
    fence_menu(main, &["Settings"]);
    let s = poll(3, || windows(t.pid, "RustDesktopIcons.Settings").first().copied());
    t.check(s.is_some(), "Settings opens the settings window");
    s.into_iter().for_each(close);
    thread::sleep(Duration::from_millis(400));
}

fn look(t: &mut Run, main: HWND) {
    let look_steps: [(&[&str], &str, Value); 10] = [
        (&["Appearance", "Opacity", "50%"], "alpha", json!(128)),
        (&["Appearance", "Icon size", "64 px"], "icon", json!(64)),
        (&["Appearance", "Show names"], "labels", json!(false)),
        (&["Appearance", "Name size", "Large"], "label_size", json!("Large")),
        (&["Appearance", "Icon spacing", "Wide"], "spacing", json!("Wide")),
        (&["Appearance", "Icon alignment", "Center"], "align", json!("Center")),
        (&["Appearance", "Chameleon mode"], "chameleon", json!(true)),
        (&["Behavior", "Automatic height"], "auto_height", json!(true)),
        (&["Open folders", "In File Explorer"], "browse", json!(false)),
        (&["Appearance", "Name color", "Automatic"], "label_color", Value::Null),
    ];
    for (path, field, want) in look_steps {
        fence_menu(main, path);
        let ok = t.until(|_| {
            let f = t.fence(1);
            if field == "alpha" { f[field].as_u64().is_some_and(|a| a.abs_diff(128) <= 2) } else { f[field] == want }
        });
        t.check(ok, &format!("{} sets {field} on this fence", path.join(" → ")));
    }
    t.check(t.fence(1)["custom"].as_array().is_some_and(|c| c.len() >= 8), "per-fence changes are kept as overrides");
    t.check(saved(&t.data)["fences"][1]["label_size"] == "Normal", "another fence is not changed");
    fence_menu(main, &["Appearance", "Color"]);
    let dialog = support::answer(t.pid, &[VK_ESCAPE]);
    t.check(dialog && wait(3, || windows(t.pid, "#32770").is_empty()), "Color… opens the color picker, Esc cancels it");
    fence_menu(main, &["Appearance", "Reset to default"]);
    t.check(
        t.until(|_| {
            let f = t.fence(1);
            f["custom"] == json!([]) && f["label_size"] == "Normal" && f["spacing"] == "Normal" && f["labels"] == true
        }),
        "Reset to default makes the fence follow the settings again",
    );
}

fn lock_and_group(t: &mut Run, main: HWND) {
    let r0 = rect(main);
    fence_menu(main, &["Behavior", "Lock position"]);
    t.check(t.until(|_| t.fence(1)["locked"] == true), "Lock position is saved");
    drag((r0.left + 80, r0.top + 12), (r0.left + 180, r0.top + 80));
    thread::sleep(Duration::from_millis(400));
    let r1 = rect(main);
    t.check((r1.left, r1.top) == (r0.left, r0.top), "a locked fence cannot be dragged");
    fence_menu(main, &["Behavior", "Lock position"]);
    t.until(|_| t.fence(1)["locked"] == false);

    fence_menu(main, &["Behavior", "Group with", "Other"]);
    t.check(
        t.until(|_| !t.fence(1)["group"].is_null() && t.fence(1)["group"] == t.fence(2)["group"]),
        "Group with puts both fences in one group",
    );
    right_click(empty_spot(main));
    menu::open();
    menu::pick(&["Behavior", "Group with"]);
    let checked = support::poll(3, || menu::checked("Other"));
    menu::close();
    t.check(checked == Some(true), "the group menu ticks the grouped fence");
    fence_menu(main, &["Behavior", "Ungroup"]);
    let ungrouped = t.until(|_| t.fence(1)["group"].is_null() && t.fence(2)["group"].is_null());
    if !ungrouped {
        println!(
            "      groups after Ungroup: {} / {}, menus open: {}",
            t.fence(1)["group"],
            t.fence(2)["group"],
            windows(0, "#32768").len()
        );
    }
    t.check(ungrouped, "Ungroup dissolves a group of two");
}
