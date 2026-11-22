use crate::support::{fence_cfg, glide, point, rect, wait};
use crate::{Ctx, click_at, double_at, empty_spot, names, open_first, shown, sleep};
use serde_json::json;
use windows::Win32::Foundation::HWND;

pub fn run(c: &mut Ctx, rolled: HWND) {
    let r0 = rect(rolled);
    let away = (r0.left + 100, r0.bottom + 450);
    glide(away, c.bar(rolled, c.sc(150)));
    sleep(900);
    let b = rect(rolled);
    c.check(b.bottom - b.top > r0.bottom - r0.top + c.sc(60), "the rolled fence opens on hover");
    c.check(shown(rolled) == names(&["R", "zhome4.txt"]), "the rolled fence shows its items");
    open_first(rolled);
    c.check(shown(rolled) == names(&["r.txt"]), "a folder opens inside the rolled fence");
    glide(empty_spot(rolled), c.bar(rolled, c.sc(30)));
    sleep(500);
    c.check(
        rect(rolled).bottom - rect(rolled).top > r0.bottom - r0.top + c.sc(60),
        "the rolled fence stays open while the mouse goes to the back arrow",
    );
    c.shot(rolled, "arrow");
    click_at(c.bar(rolled, c.sc(30)));
    c.check(shown(rolled) == names(&["R", "zhome4.txt"]), "the back arrow works in a rolled fence");
    open_first(rolled);
    double_at(c.bar(rolled, c.sc(30)));
    c.check(shown(rolled) == names(&["R", "zhome4.txt"]), "a double click on the back arrow of a rolled fence goes back");
    c.check(fence_cfg(&c.data, 4)["rolled"] == json!(true), "the fence stays a rolled fence");
    open_first(rolled);
    glide(c.bar(rolled, c.sc(150)), away);
    c.check(
        wait(5, || rect(rolled).bottom - rect(rolled).top < r0.bottom - r0.top + c.sc(10)),
        "the rolled fence rolls up when the mouse leaves",
    );
    glide(away, c.bar(rolled, c.sc(150)));
    sleep(900);
    c.check(shown(rolled) == names(&["R", "zhome4.txt"]), "rolling up brings the fence back to its own items");
    point(away);
    sleep(800);
}
