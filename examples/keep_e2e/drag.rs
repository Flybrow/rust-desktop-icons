use crate::Run;
use crate::support::{
    self,
    desk::{positions, stacked},
    waited,
};
use std::fs;

pub fn run(r: &mut Run, names: &mut Vec<String>, pid: u32) {
    match support::fences(pid).first().copied() {
        Some(f) => {
            let k = unsafe { windows::Win32::UI::HiDpi::GetDpiForSystem() } as i32;
            let (cell, cols) = (104 * k / 96, ((f.right - f.left) / (104 * k / 96)).max(1));
            let item = (f.left + ((f.right - f.left) - cols * cell) / 2 + cell / 2, f.top + (support::bar_h() + 4) * k / 96 + cell / 2);
            match support::free_desktop_spot(&[f]) {
                Some(spot) => {
                    let saved = support::cursor();
                    let before: Vec<String> = r.pins();
                    support::drag(item, spot);
                    support::point(saved);
                    let left = waited(5, || r.pins().len() == 2);
                    let gone: Vec<String> = before.iter().filter(|p| !r.pins().contains(p)).cloned().collect();
                    let out = gone
                        .first()
                        .map(|p| std::path::Path::new(p).file_name().unwrap_or_default().to_string_lossy().into_owned())
                        .unwrap_or_default();
                    r.check(left.is_some(), "drag out to the desktop: the item leaves the fence");
                    r.check(!out.is_empty() && r.desk.join(&out).exists(), "drag out to the desktop: the file stays on the Desktop");
                    let dupes = fs::read_dir(&r.desk)
                        .into_iter()
                        .flatten()
                        .flatten()
                        .filter(|e| e.file_name().to_string_lossy().contains("rdi-e2e") && e.file_name().to_string_lossy().contains(" ("))
                        .count();
                    r.check(dupes == 0, "drag out to the desktop: no duplicate file");
                    r.check(support::windows(0, "OperationStatusWindow").is_empty(), "drag out to the desktop: no Explorer error dialog");
                    let shown = waited(5, || positions().iter().any(|(p, pt)| p.ends_with(&out) && pt.x > -10_000 && !stacked(*pt)));
                    r.check(shown.is_some(), "drag out to the desktop: its icon is shown again");
                    names.retain(|n| *n != out);
                    let _ = fs::remove_file(r.desk.join(&out));
                }
                None => println!("[SKIP] drag out to the desktop: no free desktop area visible"),
            }
        }
        None => r.check(false, "fence window found for drag test"),
    }
}
