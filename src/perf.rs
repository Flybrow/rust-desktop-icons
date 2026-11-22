use std::{cell::RefCell, io::Write, sync::LazyLock};

static ON: LazyLock<bool> = LazyLock::new(|| std::env::var_os("RDI_PERF").is_some());

thread_local! {
    static LOG: RefCell<Vec<String>> = const { RefCell::new(vec![]) };
}

pub fn on() -> bool {
    *ON
}

pub fn mark(what: &str, h: isize, ms: f64) {
    if !*ON {
        return;
    }
    let line = format!("{:.3} {h} {what} {ms:.3}", crate::win::now_ms());
    let full = LOG.with(|l| {
        let mut l = l.borrow_mut();
        l.push(line);
        l.len() >= 64
    });
    if full {
        flush();
    }
}

pub fn span<R>(what: &str, h: isize, f: impl FnOnce() -> R) -> R {
    if !*ON {
        return f();
    }
    let t = crate::win::now_ms();
    let r = f();
    mark(what, h, crate::win::now_ms() - t);
    r
}

pub fn flush() {
    if !*ON {
        return;
    }
    let lines = LOG.with(|l| std::mem::take(&mut *l.borrow_mut()));
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(crate::report::rotated("perf.log")) {
        let _ = f.write_all((lines.join("\n") + "\n").as_bytes());
    }
}
