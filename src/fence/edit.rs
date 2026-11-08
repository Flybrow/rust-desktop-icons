use super::{cell, label_font, layout, metrics, reload, title_font, update, view};
use crate::domain::side::Side;
use crate::{app::with, domain::names, i18n::T, report, shell, store, win::*};
use std::{fs, path::PathBuf};
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::{ClientToScreen, HFONT},
    UI::{
        Input::KeyboardAndMouse::{VK_ESCAPE, VK_RETURN},
        WindowsAndMessaging::*,
    },
};

pub fn key(msg: &MSG) -> bool {
    let k = msg.wParam.0 as u16;
    if msg.message != WM_KEYDOWN || (k != VK_RETURN.0 && k != VK_ESCAPE.0) {
        return false;
    }
    let Some(parent) = with(|a| a.views.iter().find(|v| v.edit.as_ref().is_some_and(|e| e.0 == msg.hwnd)).map(|v| v.hwnd)).flatten() else {
        return false;
    };
    finish(parent, k == VK_RETURN.0);
    true
}

fn open(h: HWND, r: RECT, text: &str, font: HFONT, target: Option<PathBuf>) {
    let mut p = POINT { x: r.left, y: r.top };
    let _ = unsafe { ClientToScreen(h, &mut p) };
    let at = RECT { left: p.x, top: p.y, right: p.x + r.right - r.left, bottom: p.y + r.bottom - r.top };
    if let Some(e) = edit_popup(h, at, text, font, true) {
        view(h, |v| v.edit = Some((e, target)));
    }
}

pub(super) fn title(h: HWND) {
    let Some((text, font)) = with(|a| Some((a.fence_of(h)?.active().title.clone(), title_font(a, h)))).flatten() else { return };
    let (g, pad, wide) = (layout::geo_now(h), scale(h, 5), scale(h, 220));
    let (x0, y0, x1, _) = g.bar_rect();
    let (left, right) = match g.side {
        Side::Left => (x0 + pad, x0 + wide),
        Side::Right => (x1 - wide, x1 - pad),
        _ => (x0 + pad, x1 - pad),
    };
    open(h, RECT { left, top: y0 + pad, right, bottom: y0 + g.bar - pad }, &text, font, None);
}

pub(super) fn item(h: HWND, path: &PathBuf) {
    if crate::domain::system::Sys::of(path).is_some() {
        return;
    }
    let s = |v| scale(h, v);
    let (cell, icon) = metrics(h);
    let g = layout::geo_now(h);
    let Some((text, font, origin)) = with(|a| {
        let font = label_font(a, h);
        let flow = layout::flow(a, h, g.grid_w(), cell)?;
        let v = a.view(h)?;
        let i = v.items.iter().position(|it| &it.path == path)?;
        let (x, y) = flow.origin(i);
        Some((String::from_utf16_lossy(&v.items[i].name), font, g.cell((x, y - v.scroll), cell)))
    })
    .flatten() else {
        return;
    };
    let r = cell::label_at(origin, cell, icon, s);
    open(
        h,
        RECT { top: r.top - s(2), bottom: r.top + s(22), left: r.left - s(10), right: r.right + s(10) },
        &text,
        font,
        Some(path.clone()),
    );
}

pub(super) fn finish(h: HWND, commit: bool) {
    crate::undo::step(|| commit_edit(h, commit));
}

fn commit_edit(h: HWND, commit: bool) {
    let Some((e, target)) = with(|a| a.view(h)?.edit.take()).flatten() else {
        return;
    };
    let text = take_text(e);
    if !commit {
        return;
    }
    match target {
        None if !text.trim().is_empty() => {
            update(h, |f| f.active_mut().title = text.trim().chars().filter(|c| !c.is_control()).take(64).collect())
        }
        Some(path) => {
            let shown = with(|a| a.view(h)?.items.iter().find(|i| i.path == path).map(|i| String::from_utf16_lossy(&i.name))).flatten();
            let file = store::name(&path);
            if let Some(name) = names::renamed(&file, shown.as_deref().unwrap_or(&file), &text) {
                let dest = path.with_file_name(&name);
                let result = if dest.exists() && !shell::same_path(&dest, &path) {
                    Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists))
                } else {
                    fs::rename(&path, &dest)
                };
                match result {
                    Ok(()) => {
                        crate::undo::moved(&path, &dest);
                        reload(h)
                    }
                    Err(err) => report::alert(T::ErrRename, &format!("{file} → {name} ({err})")),
                }
            }
        }
        None => {}
    }
}
