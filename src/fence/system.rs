use super::{arrange, bind, drop, fence_of, ghost, items, tabs};
use crate::{
    app::{change, desk_icons, with},
    domain::{Tab, system::Sys},
    shell,
};
use std::path::PathBuf;
use windows::Win32::{Foundation::HWND, System::Com::IDataObject};

fn place(list: &[Sys], tab: Option<u64>, shown: bool) {
    change(|c| list.iter().for_each(|s| c.place(*s, tab)));
    for &s in list {
        match (tab, shown) {
            (Some(_), _) => desk_icons::hide(s),
            (None, true) => desk_icons::show(s),
            (None, false) => desk_icons::restore(s),
        }
    }
    items::windows().into_iter().for_each(bind);
}

fn paths(list: &[Sys]) -> Vec<PathBuf> {
    list.iter().map(|s| s.path()).collect()
}

pub(super) fn toggle(h: HWND, s: Sys) {
    if let Some(t) = fence_of(h).map(|f| f.active().clone()) {
        place(&[s], (!t.system.contains(&s)).then_some(t.id), false);
    }
}

pub(super) fn released(tabs: &[Tab]) {
    tabs.iter().flat_map(|t| &t.system).for_each(|s| desk_icons::restore(*s));
}

pub(super) fn drag(h: HWND, at: (i32, i32), list: Vec<Sys>) {
    with(|a| a.sys_drag = list.clone());
    let image = items::index_at(h, at).and_then(|i| ghost::image(h, i, at));
    let landed = shell::drag_out(h, &paths(&list), image, true, true);
    if with(|a| !std::mem::take(&mut a.sys_drag).is_empty()) == Some(true) && landed {
        place(&list, None, true);
    }
}

pub(super) fn dropped(h: HWND, data: Option<&IDataObject>, pt: (i32, i32)) -> bool {
    let ours = with(|a| std::mem::take(&mut a.sys_drag)).unwrap_or_default();
    let list = if ours.is_empty() { data.map(shell::system::in_data).unwrap_or_default() } else { ours };
    if list.is_empty() {
        return false;
    }
    match drop::target_tab(h, pt) {
        Some(tab) if list.iter().all(|s| tab.system.contains(s)) && tabs::at(h, pt).is_none() => {
            arrange::move_within(h, &paths(&list), items::item_at(h, pt))
        }
        Some(tab) => place(&list, Some(tab.id), false),
        None => {}
    }
    true
}
