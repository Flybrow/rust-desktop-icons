use crate::{
    app::{change, with},
    dialog::{self, Button, Spec},
    i18n::T,
    update::plain,
};

const NOTES: &str = include_str!("../.github/release-notes.md");
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn upgraded(previous: Option<&str>, existed: bool) -> bool {
    existed && previous != Some(VERSION)
}

fn split(notes: &str) -> (String, String) {
    let text = plain(notes);
    match text.split_once('\n') {
        Some((title, body)) => (title.trim().to_string(), body.trim().to_string()),
        None => (text, String::new()),
    }
}

pub fn show(existed: bool) {
    let mut shown = false;
    change(|c| {
        shown = upgraded(c.version.as_deref(), existed);
        c.version = Some(VERSION.into());
    });
    if shown {
        let (title, body) = split(NOTES);
        let buttons = vec![Button { label: with(|a| a.t(T::GotIt).to_string()).unwrap_or_default(), act: dialog::close }];
        dialog::show(Spec { title, body, buttons, progress: None, dismiss: dialog::close });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes() {
        assert!(upgraded(None, true) && upgraded(Some("0.1.0"), true));
        assert!(!upgraded(Some(VERSION), true) && !upgraded(None, false));
        assert_eq!(split("## What's new in X\n\n- **A**\n- B"), ("What's new in X".into(), "•  A\n•  B".into()));
        assert_eq!(split("Only"), ("Only".into(), String::new()));
        assert!(!split(NOTES).0.is_empty() && !split(NOTES).1.is_empty());
    }
}
