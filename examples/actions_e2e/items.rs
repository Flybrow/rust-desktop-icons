use crate::support::{self, chord, click, clip, close, explorer, menu, poll, right_click, type_text, wait};
use crate::{Run, empty_spot, first_item, pasted};
use std::{fs, thread, time::Duration};
use windows::Win32::{
    Foundation::HWND,
    UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_CONTROL, VK_DELETE, VK_ESCAPE, VK_F11, VK_MENU, VK_RETURN, VK_SHIFT, VK_V},
};

const PEEK: [VIRTUAL_KEY; 4] = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_F11];

pub fn run(t: &mut Run, other: HWND) {
    if let Some(h) = t.hwnd("Mover") {
        right_click(first_item(h));
        menu::open();
        menu::pick(&["Move to", "Other"]);
        t.check(
            wait(5, || t.dir(2).join("move-me.txt").exists() && !t.dir(3).join("move-me.txt").exists()),
            "Move to → another fence moves the file",
        );
    }
    let (desk, out_name) = (support::desk::desktop(), t.out_name());
    if let Some(h) = t.hwnd("Out") {
        right_click(first_item(h));
        menu::open();
        menu::pick(&["Move to", "Desktop"]);
        t.check(
            wait(5, || desk.join(&out_name).exists() && !t.dir(4).join(&out_name).exists()),
            "Move to → Desktop puts the file back on the desktop",
        );
        let _ = fs::remove_file(desk.join(&out_name));
    }
    if let Some(h) = t.hwnd("Del") {
        click(first_item(h));
        thread::sleep(Duration::from_millis(300));
        chord(&[VK_SHIFT, VK_DELETE]);
        support::answer(t.pid, &[VK_RETURN]);
        t.check(wait(5, || !t.dir(5).join("delete-me.txt").exists()), "Shift+Delete deletes the selected item (after Windows asks)");
    }

    let paste_src = t.paste_src();
    t.check(clip::set_files(&[paste_src.as_path()]), "a file is put on the clipboard");
    click(empty_spot(other));
    thread::sleep(Duration::from_millis(300));
    chord(&[VK_CONTROL, VK_V]);
    t.check(wait(5, || pasted(&t.dir(2))), "Ctrl+V in a fence adds the copied file (as a shortcut)");
    t.check(paste_src.exists(), "the pasted original stays where it was");

    chord(&PEEK);
    thread::sleep(Duration::from_millis(500));
    type_text("box");
    thread::sleep(Duration::from_millis(500));
    chord(&[VK_RETURN]);
    let found = poll(5, || explorer("Box"));
    t.check(found.is_some(), "Enter in the search opens the found item");
    found.into_iter().for_each(close);
    chord(&[VK_ESCAPE]);
    thread::sleep(Duration::from_millis(500));
}
