use crate::{
    app::{change, with},
    dialog::{self, Button, Spec},
    i18n::LANGS,
    i18n::T,
    update::notes::{Notes, parse},
};

const NOTES: &str = include_str!("../.github/release-notes.md");
const NOTES_FR: &str = include_str!("../.github/release-notes.fr.md");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASES: &str = "https://github.com/Flybrow/rust-desktop-icons/releases/tag/v";

fn read_all() {
    crate::shell::open(std::path::Path::new(&format!("{RELEASES}{VERSION}")));
    dialog::close();
}

pub fn upgraded(previous: Option<&str>, existed: bool) -> bool {
    existed && previous != Some(VERSION)
}

fn notes(lang: &str) -> Notes {
    parse(if lang == "fr" { NOTES_FR } else { NOTES })
}

pub fn show(existed: bool) {
    let mut shown = false;
    change(|c| {
        shown = upgraded(c.version.as_deref(), existed);
        c.version = Some(VERSION.into());
    });
    if shown {
        let Notes { title, blocks } = notes(with(|a| LANGS[a.lang()].0).unwrap_or("en"));
        let label = |k| with(|a| a.t(k).to_string()).unwrap_or_default();
        let buttons = vec![Button { label: label(T::GotIt), act: dialog::close }, Button { label: label(T::ReadAll), act: read_all }];
        dialog::show(Spec { title, body: String::new(), buttons, progress: None, dismiss: dialog::close, notes: blocks });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_in_two_languages() {
        assert!(upgraded(None, true) && upgraded(Some("0.1.0"), true));
        assert!(!upgraded(Some(VERSION), true) && !upgraded(None, false));
        let (en, fr) = (notes("en"), notes("fr"));
        assert!(en.title.contains(VERSION) && fr.title.contains(VERSION) && en.title != fr.title);
        assert_eq!(en.blocks.len(), fr.blocks.len());
        assert_eq!(notes("de"), en);
    }
}
