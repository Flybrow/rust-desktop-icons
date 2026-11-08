use super::{
    act::{choose, pick},
    hwnd, refresh,
};
use crate::{
    app::{change, with},
    domain::matching::{AGES, Expire, NameRule},
    i18n::T,
    prompt,
};
use windows::Win32::Foundation::HWND;

type Place = (u64, u64);

fn target(h: HWND, except: Option<Place>, current: Option<Place>) -> Option<Place> {
    let list: Vec<(u64, u64, String)> =
        with(|a| a.cfg.targets()).unwrap_or_default().into_iter().filter(|(f, t, _)| Some((*f, *t)) != except).collect();
    choose(h, list.iter().map(|(f, t, n)| (n.clone(), Some((*f, *t)) == current)).collect()).map(|i| (list[i].0, list[i].1))
}

fn days(h: HWND, current: u32) -> Option<u32> {
    pick(h, &AGES, current, |d| with(|a| a.t(T::OlderThan).replace("{}", &d.to_string())).unwrap_or_default())
}

fn edit_name(i: usize, f: impl FnOnce(&mut Vec<NameRule>)) {
    change(|c| {
        if i < c.name_rules.len() {
            f(&mut c.name_rules)
        }
    });
}

fn ask_pattern(initial: &str, done: impl FnOnce(String) + 'static) {
    prompt::ask(initial, move |p| {
        done(p.trim().to_string());
        refresh();
    });
}

pub(super) fn add_name() {
    ask_pattern("*.pdf", |pattern| {
        let Some(h) = hwnd() else { return };
        if let Some((fence, tab)) = target(h, None, None) {
            change(|c| c.name_rules.push(NameRule { pattern, fence, tab }));
        }
    });
}

pub(super) fn name(h: HWND, i: usize) {
    let Some((rule, labels)) =
        with(|a| Some((a.cfg.name_rules.get(i)?.clone(), [T::EditPattern, T::ChangeFence, T::DeleteRule].map(|t| a.t(t))))).flatten()
    else {
        return;
    };
    match choose(h, labels.iter().map(|l| (l.to_string(), false)).collect()) {
        Some(0) => ask_pattern(&rule.pattern, move |p| edit_name(i, |r| r[i].pattern = p)),
        Some(1) => {
            if let Some((f, t)) = target(h, None, Some((rule.fence, rule.tab))) {
                edit_name(i, |r| (r[i].fence, r[i].tab) = (f, t));
            }
        }
        Some(2) => edit_name(i, |r| drop(r.remove(i))),
        _ => {}
    }
}

fn set_age(src: Place, e: Option<Expire>) {
    change(|c| c.tab_mut(src.0, src.1).iter_mut().for_each(|t| t.expire = e));
}

pub(super) fn add_age(h: HWND) {
    let Some(src) = target(h, None, None) else { return };
    let Some(days) = days(h, 30) else { return };
    if let Some((fence, tab)) = target(h, Some(src), None) {
        set_age(src, Some(Expire { days, fence, tab }));
    }
}

pub(super) fn age(h: HWND, fence: u64, tab: u64) {
    let Some((e, labels)) = with(|a| {
        let e = a.cfg.tab(fence, tab)?.expire?;
        Some((e, [a.t(T::OlderThan).replace("{}", &e.days.to_string()), a.t(T::ChangeFence).into(), a.t(T::DeleteRule).into()]))
    })
    .flatten() else {
        return;
    };
    let src = (fence, tab);
    match choose(h, labels.into_iter().map(|l| (l, false)).collect()) {
        Some(0) => days(h, e.days).iter().for_each(|&days| set_age(src, Some(Expire { days, ..e }))),
        Some(1) => {
            target(h, Some(src), Some((e.fence, e.tab))).iter().for_each(|&(fence, tab)| set_age(src, Some(Expire { fence, tab, ..e })))
        }
        Some(2) => set_age(src, None),
        _ => {}
    }
}
