#![windows_subsystem = "windows"]

mod app;
mod backup;
mod dialog;
mod domain;
mod fence;
mod i18n;
mod keep;
mod layered;
mod perf;
mod prefs;
mod prompt;
mod render;
mod report;
mod rules;
mod search;
mod settings;
mod shell;
mod store;
mod tray;
mod undo;
mod uninstall;
mod update;
mod visibility;
mod whatsnew;
mod win;

fn main() {
    unsafe { std::env::remove_var("ELECTRON_RUN_AS_NODE") };
    app::run();
}
