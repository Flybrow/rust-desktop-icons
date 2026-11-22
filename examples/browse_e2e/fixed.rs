use crate::support::{chord, fence_cfg, point, rect, wait};
use crate::{Ctx, click_at, double_at, names, open_first, shown, sleep};
use serde_json::json;
use std::fs;
use windows::Win32::{
    Foundation::HWND,
    UI::Input::KeyboardAndMouse::{VK_BACK, VK_CONTROL, VK_SHIFT, VK_TAB},
};

pub fn run(c: &mut Ctx, fixed: HWND) {
    let home3 = names(&["A", "zhome.txt"]);

    let first = shown(fixed);
    c.check(first == home3, &format!("the fixed fence shows its tab ({first:?})"));
    open_first(fixed);
    c.check(wait(3, || shown(fixed) == names(&["B", "za.txt"])), "Enter on a folder opens it inside the fence");
    open_first(fixed);
    c.check(shown(fixed) == names(&["deep.txt"]), "a folder inside it opens too");
    c.shot(fixed, "deep");
    let width = rect(fixed).right - rect(fixed).left;
    for x in (c.sc(20)..width - c.sc(20)).rev().step_by(12) {
        point(c.bar(fixed, x));
        sleep(10);
    }
    c.shot(fixed, "arrow_hovered");
    double_at(c.bar(fixed, c.sc(30)));
    for x in [40, 60, 80, 60, 30] {
        point(c.bar(fixed, c.sc(x)));
        sleep(60);
    }
    point((rect(fixed).left + c.sc(60), rect(fixed).top + c.sc(80)));
    sleep(300);
    c.check(
        shown(fixed) == home3,
        "after sliding along the bar, a double click on the back arrow goes up two levels and stays on that tab",
    );
    c.check(fence_cfg(&c.data, 1)["rolled"] != json!(true), "a double click on the back arrow never rolls the fence up");
    open_first(fixed);
    click_at(c.bar(fixed, width / 2));
    c.check(shown(fixed) == names(&["two.txt"]), "clicking another tab while in a folder shows that tab");
    click_at(c.bar(fixed, width - c.sc(30)));
    c.check(shown(fixed) == home3, "the tab left in a folder comes back to its own items");
    open_first(fixed);
    chord(&[VK_CONTROL, VK_TAB]);
    sleep(600);
    c.check(shown(fixed) == names(&["one.txt"]), "Ctrl+Tab while in a folder goes to the next tab");
    chord(&[VK_CONTROL, VK_SHIFT, VK_TAB]);
    sleep(600);
    c.check(shown(fixed) == home3, "Ctrl+Shift+Tab comes back to the tab's own items");
    open_first(fixed);
    open_first(fixed);
    chord(&[VK_BACK]);
    sleep(600);
    c.check(shown(fixed) == names(&["B", "za.txt"]), "Backspace goes up one level");
    fs::remove_dir_all(c.dir(3).join("A")).unwrap();
    sleep(1500);
    c.check(shown(fixed).is_empty(), "a folder deleted while open shows empty");
    click_at(c.bar(fixed, c.sc(30)));
    c.check(shown(fixed) == names(&["zhome.txt"]), "the back arrow still works after the folder is deleted");
}
