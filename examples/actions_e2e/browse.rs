use crate::support::{self, chord, click, close, double_click, explorer, menu, poll, rect, right_click, type_text, wait, windows};
use crate::{Run, empty_spot, first_item, pasted, point_and_ctrl_double};
use std::{thread, time::Duration};
use windows::Win32::{
    Foundation::HWND,
    UI::Input::KeyboardAndMouse::{VK_A, VK_CONTROL, VK_F2, VK_RETURN, VK_V},
};

pub fn run(t: &mut Run, main: HWND) {
    point_and_ctrl_double(first_item(main));
    let found = poll(5, || explorer("Box"));
    t.check(found.is_some(), "Ctrl+double-click on a folder opens it in Explorer");
    found.into_iter().for_each(close);
    thread::sleep(Duration::from_millis(500));
    let r = rect(main);
    double_click(first_item(main));
    thread::sleep(Duration::from_millis(800));
    t.check(explorer("Box").is_none(), "a plain double-click does not open Explorer");
    click(empty_spot(main));
    thread::sleep(Duration::from_millis(300));
    chord(&[VK_CONTROL, VK_V]);
    t.check(wait(5, || pasted(&t.dir(1).join("Box"))), "the folder opened inside the fence receives what is pasted there");
    click((r.left + 20, r.top + 12));
    thread::sleep(Duration::from_millis(600));
    rename_first(main, "Box2");
    t.check(wait(3, || t.dir(1).join("Box2").is_dir()), "the title-bar arrow goes back to the fence's own items");
    right_click(first_item(main));
    menu::open();
    menu::pick(&["Open in the fence"]);
    thread::sleep(Duration::from_millis(600));
    click(empty_spot(main));
    chord(&[VK_CONTROL, VK_V]);
    t.check(wait(5, || pasted(&t.dir(1).join("Box2"))), "Open in the fence (folder right-click) opens the folder inside the fence");
    click((r.left + 20, r.top + 12));
    rolled(t);
}

fn rename_first(h: HWND, name: &str) {
    click(first_item(h));
    thread::sleep(Duration::from_millis(300));
    chord(&[VK_F2]);
    thread::sleep(Duration::from_millis(300));
    chord(&[VK_CONTROL, VK_A]);
    type_text(name);
    chord(&[VK_RETURN]);
}

fn rolled(t: &mut Run) {
    let w = t.w;
    if let Some(h) = windows(t.pid, "RustDesktopIcons.Fence").into_iter().find(|h| (rect(*h).top - (w.top + 600)).abs() < 40) {
        let bar = rect(h);
        let tall = || rect(h).bottom - rect(h).top > 100;
        support::point((bar.left + 150, bar.top + 15));
        let opened = wait(4, tall);
        thread::sleep(Duration::from_millis(500));
        double_click(first_item(h));
        thread::sleep(Duration::from_millis(800));
        support::point((w.right - 20, w.bottom - 20));
        let rolled = wait(5, || !tall());
        thread::sleep(Duration::from_millis(800));
        support::point((bar.left + 150, bar.top + 15));
        wait(4, tall);
        thread::sleep(Duration::from_millis(600));
        rename_first(h, "Deep2");
        t.check(
            opened && rolled && wait(3, || t.dir(7).join("Deep2").is_dir()),
            "a rolled fence that rolls up goes back to its own folder",
        );
        support::point((w.right - 20, w.bottom - 20));
    } else {
        t.check(false, "the rolled fence is shown");
    }
}
