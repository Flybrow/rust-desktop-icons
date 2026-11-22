use crate::support::{self, chord, explorer, menu, rect, right_click};
use crate::{Ctx, click_at, empty_spot, names, open_first, shown, sleep};
use windows::Win32::{Foundation::HWND, UI::Input::KeyboardAndMouse::VK_BACK};

pub fn run(c: &mut Ctx, side: HWND, linking: HWND) {
    c.check(shown(side) == names(&["V", "zhome5.txt"]), "the side-bar fence shows its items");
    open_first(side);
    c.check(shown(side) == names(&["v.txt"]), "a folder opens in a fence with its bar on the side");
    c.shot(side, "folder");
    let s = rect(side);
    click_at((s.left + c.sc(12), s.top + c.sc(30)));
    c.check(shown(side) == names(&["V", "zhome5.txt"]), "the start of a side bar is the back arrow");

    let home6 = names(&["alink.lnk", "zhome6.txt"]);
    c.check(shown(linking) == home6, "the fence shows its folder shortcut");
    open_first(linking);
    c.check(shown(linking) == names(&["in.txt"]), "Enter on a folder shortcut opens its folder inside the fence");
    c.check(explorer("linked").is_none(), "a folder shortcut does not open File Explorer");
    chord(&[VK_BACK]);
    sleep(600);
    c.check(shown(linking) == home6, "Backspace comes back from a folder shortcut");
    let l = rect(linking);
    let item = (l.left + c.sc(60), l.top + c.sc(75));
    click_at(item);
    right_click(item);
    c.check(menu::open() && menu::pick(&["Open in the fence"]), "a folder shortcut's right-click menu offers Open in the fence");
    sleep(600);
    c.check(shown(linking) == names(&["in.txt"]), "Open in the fence on a folder shortcut opens its folder inside the fence");
    c.shot(linking, "shortcut");
    open_first(side);
    c.check(shown(side) == names(&["v.txt"]), "a folder opens again in the side-bar fence");
    click_at(empty_spot(linking));
    c.check(
        shown(side) == names(&["V", "zhome5.txt"]),
        "Back to the start when clicking elsewhere: the folder closes when another fence is clicked",
    );
    c.check(shown(linking) == names(&["in.txt"]), "without that option, a folder stays open when clicking elsewhere");
    match support::free_desktop_spot(&support::fences(c.pid)) {
        Some(p) => {
            open_first(side);
            c.check(shown(side) == names(&["v.txt"]), "the folder opens once more");
            click_at(p);
            c.check(
                shown(side) == names(&["V", "zhome5.txt"]),
                "Back to the start when clicking elsewhere: a click on the desktop closes it too",
            );
        }
        None => println!("[SKIP] no empty desktop spot visible"),
    }
}
