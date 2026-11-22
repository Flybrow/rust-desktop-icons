use crate::support::{
    self,
    desk::{cover, hidden, positions, screen, stacked},
    waited,
};
use crate::{EXT, Run, file};
use std::{fs, thread, time::Duration};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};

pub fn run(r: &mut Run, names: &[String], pid: u32) {
    let loose = file("rdi-e2e-loose");
    fs::write(r.desk.join(&loose), "Rust Desktop Icons e2e test file").unwrap();
    let spot = waited(10, || positions().iter().any(|(p, pt)| p.ends_with(&loose) && pt.x > -10_000 && !stacked(*pt)))
        .and_then(|_| positions().into_iter().find(|(p, _)| p.ends_with(&loose)))
        .map(|(_, pt)| screen(pt));
    match (spot, support::free_desktop_spot(&support::fences(pid))) {
        (Some(from), Some(to)) => {
            let watched = names.to_vec();
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let flag = stop.clone();
            let probe = thread::spawn(move || {
                unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
                let (mut least, mut seen) = (watched.len(), String::new());
                while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                    let n = hidden(&watched);
                    if n < least {
                        least = n;
                        let at: Vec<_> = positions().into_iter().filter(|(p, _)| p.ends_with(EXT)).map(|(p, pt)| (p, screen(pt))).collect();
                        seen = format!("cover {:?}, icons {at:?}", cover().map(|r| (r.left, r.top, r.right, r.bottom)));
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                (least, seen)
            });
            let saved = support::cursor();
            support::drag((from.0 + 20, from.1 + 20), to);
            support::point(saved);
            thread::sleep(Duration::from_millis(1500));
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            let (least, seen) = probe.join().unwrap();
            if least < names.len() {
                println!("      when shown: {seen}");
            }
            println!("      moving a loose icon: fewest hidden = {least}/{}", names.len());
            r.check(least == names.len(), "moving a loose desktop icon never shows the hidden icons");
        }
        _ => println!("[SKIP] moving a loose icon: icon or free spot not found"),
    }
    let _ = fs::remove_file(r.desk.join(&loose));
}
