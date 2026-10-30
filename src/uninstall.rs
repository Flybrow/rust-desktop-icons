use crate::{
    i18n::{self, T},
    report,
    shell::{self, hide::Desk},
    store,
    win::msgbox,
};
use windows::Win32::{
    System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
    UI::WindowsAndMessaging::{MB_ICONWARNING, MB_OK},
};

pub fn run() {
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let (cfg, _) = store::load();
    crate::app::desk_icons::restore_all(&cfg);
    if cfg.desk_flags.is_some() {
        match Desk::open() {
            Some(d) => {
                d.sync(&[], None);
                cfg.desk_flags.iter().for_each(|f| d.set_flags(*f));
            }
            None => report::log("uninstall: cannot reach the desktop view to show hidden icons"),
        }
    }
    let Some(desk) = shell::desktop() else { return };
    let errors: Vec<String> = store::tab_dirs().iter().flat_map(|d| store::evacuate(d, &desk).1).collect();
    if errors.is_empty() && store::tab_dirs().is_empty() {
        let _ = std::fs::remove_dir_all(store::root());
        return;
    }
    let lang = i18n::resolve(cfg.lang.as_deref(), &shell::locale());
    let text = i18n::get(lang, T::ErrDelete).replace("{}", &errors.iter().take(10).cloned().collect::<Vec<_>>().join("\n"));
    report::log(&text);
    msgbox(None, &text, MB_OK | MB_ICONWARNING, i18n::rtl(lang));
}
