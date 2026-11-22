use crate::support::desk::{MASK, flags, hidden, positions};
use crate::{Run, file};
use std::{
    fs,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn restart_explorer() {
    let _ = Command::new("taskkill").args(["/f", "/im", "explorer.exe"]).stdout(Stdio::null()).stderr(Stdio::null()).status();
    let _ = Command::new("explorer.exe").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

pub fn run(r: &mut Run, names: &[String]) -> u32 {
    let control = file("rdi-e2e-control");
    fs::write(r.desk.join(&control), "Rust Desktop Icons e2e test file").unwrap();
    restart_explorer();
    let start = Instant::now();
    let (mut shown, mut t) = (None, None);
    while t.is_none() && start.elapsed() < Duration::from_secs(60) {
        let listed = positions().iter().any(|(p, _)| p.ends_with(&control));
        let all = hidden(names) == names.len();
        if !all && shown.is_none() {
            shown = Some(start.elapsed());
        }
        if listed && all {
            t = Some(start.elapsed());
        }
        thread::sleep(Duration::from_millis(10));
    }
    thread::sleep(Duration::from_millis(4000));
    let before = r.config()["desk_flags"].as_u64().map_or_else(flags, |f| f as u32 & MASK);
    let visible = shown.zip(t).map_or(0, |(s, e)| (e - s).as_millis());
    println!("      explorer restart: hidden icons visible for {visible} ms (first seen at {shown:?}, all hidden at {t:?})");
    r.check(t.is_some() && hidden(names) == names.len(), "icons hidden again after Explorer restart (and stay hidden)");
    let _ = fs::remove_file(r.desk.join(&control));
    before
}
