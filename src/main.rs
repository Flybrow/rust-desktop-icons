#![windows_subsystem = "windows"]

mod app;
mod dialog;
mod domain;
mod fence;
mod i18n;
mod keep;
mod layered;
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
mod uninstall;
mod update;
mod visibility;
mod whatsnew;
mod win;

fn main() {
    app::run();
}
