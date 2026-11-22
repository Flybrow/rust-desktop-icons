use super::{DESCRIBED, HEADER, NOTE, Ui, f};
use crate::support;
use crate::support::chord;
use serde_json::json;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_ESCAPE, VK_RETURN, VK_Z};

pub fn icons(u: &mut Ui) {
    let icons = [HEADER, NOTE, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED];
    u.page(1);
    u.row(&icons, 4);
    u.check(u.until(|c| c["look"]["thumbs"] == false && f(c, 1, "thumbs") == false), "Icons → Show thumbnails can be turned off");
    u.row(&icons, 4);
    u.until(|c| c["look"]["thumbs"] == true);
    u.row(&icons, 5);
    let list = u.until(|c| c["look"]["list"] == true && f(c, 1, "list") == true);
    u.row(&icons, 5);
    u.check(list && u.until(|c| c["look"]["list"] == false), "Icons → Show as a list toggles for every fence");
    u.row(&icons, 8);
    u.pick(&["Wide"]);
    u.check(
        u.until(|c| c["look"]["spacing"] == "Wide" && f(c, 1, "spacing") == "Wide" && f(c, 0, "spacing") == "Compact"),
        "Icons → Icon spacing applies to every fence except one that changed it",
    );
    u.row(&icons, 6);
    u.pick(&["Large"]);
    u.check(u.until(|c| c["look"]["label_size"] == "Large" && f(c, 0, "label_size") == "Large"), "Icons → Name size applies to the fences");
    u.row(&icons, 3);
    u.check(u.until(|c| c["look"]["labels"] == false && f(c, 1, "labels") == false), "Icons → Show names can be turned off");
    u.row(&icons, 3);
    u.until(|c| c["look"]["labels"] == true);
    u.row(&icons, 7);
    u.pick(&["Color"]);
    let dialog = support::answer(u.pid, &[VK_ESCAPE]);
    u.check(dialog && u.cfg()["look"]["label_color"].is_null(), "Icons → Name color opens the color picker; cancel keeps it automatic");
    u.row(&icons, 9);
    u.pick(&["Center"]);
    u.check(u.until(|c| c["look"]["align"] == "Center" && f(c, 0, "align") == "Center"), "Icons → Icon alignment applies to the fences");
    u.row(&icons, 2);
    u.pick(&["64 px"]);
    u.check(u.until(|c| c["look"]["icon"] == 64 && f(c, 1, "icon") == 64), "Icons → Icon size applies to the fences");
}

pub fn look(u: &mut Ui) {
    let look = [HEADER, NOTE, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED];
    u.page(0);
    u.row(&look, 3);
    u.check(
        u.until(|c| c["look"]["wallpaper"] == true && f(c, 1, "wallpaper") == true),
        "Default appearance → Color from the wallpaper turns on",
    );
    u.row(&look, 3);
    u.until(|c| c["look"]["wallpaper"] == false);
    u.row(&look, 5);
    u.pick(&["Square"]);
    u.check(
        u.until(|c| c["look"]["corner"] == "Square" && f(c, 1, "corner") == "Square"),
        "Default appearance → Corners apply to the fences",
    );
    u.row(&look, 4);
    u.pick(&["70%"]);
    u.check(u.until(|c| c["look"]["alpha"].as_u64().is_some_and(|a| a.abs_diff(178) <= 2)), "Default appearance → Opacity is saved");
    u.row(&look, 6);
    u.pick(&["None"]);
    u.check(u.until(|c| c["look"]["tint"].is_null()), "Default appearance → Icon tint None");
    u.row(&look, 7);
    let asked = support::answer(u.pid, &[VK_RETURN]);
    u.check(
        asked && u.until(|c| f(c, 0, "custom") == json!([]) && f(c, 0, "spacing") == "Wide"),
        "Reset all fences asks, then every fence follows the settings",
    );
}

pub fn bar(u: &mut Ui) {
    let bar = [HEADER, NOTE, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED];
    u.page(2);
    u.row(&bar, 2);
    u.pick(&["Left"]);
    u.check(
        u.until(|c| c["look"]["title_align"] == "Left" && f(c, 1, "title_align") == "Left"),
        "Title bar → alignment applies to the fences",
    );
    u.row(&bar, 3);
    u.pick(&["Large"]);
    u.check(u.until(|c| c["look"]["title_size"] == "Large"), "Title bar → title size is saved");
    u.row(&bar, 4);
    u.pick(&["Automatic"]);
    u.check(u.until(|c| c["look"]["bar_color"].is_null()), "Title bar → bar color stays automatic");
    u.row(&bar, 5);
    u.check(u.until(|c| c["look"]["bar_hover"] == true && f(c, 1, "bar_hover") == true), "Title bar → show on hover turns on");
    u.row(&bar, 5);
    u.until(|c| c["look"]["bar_hover"] == false);
    chord(&[VK_CONTROL, VK_Z]);
    u.check(u.until(|c| c["look"]["bar_hover"] == true), "Ctrl+Z in the settings undoes the last change");
}
