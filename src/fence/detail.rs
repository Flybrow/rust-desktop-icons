use super::cell::{self, Cell, LINE, PAD};
use crate::{
    app::Item,
    domain::columns::{Col, Mode, Span},
    shell::details::Info,
};
use windows::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{DRAW_TEXT_FORMAT, DT_RIGHT},
};

pub(super) struct Prefix {
    pub modified: String,
    pub size: String,
}

fn field(i: &Info, c: Col) -> &[u16] {
    match c {
        Col::Name => &[],
        Col::Modified => &i.modified,
        Col::Created => &i.created,
        Col::Type => &i.kind,
        Col::Size => &i.size,
    }
}

fn labeled(prefix: &str, value: &[u16]) -> Vec<u16> {
    if value.is_empty() {
        return vec![];
    }
    let mut out: Vec<u16> = format!("{prefix}: ").encode_utf16().collect();
    out.extend_from_slice(value);
    out
}

fn below(r: RECT, n: i32, line: i32) -> RECT {
    RECT { top: r.top + n * line, bottom: r.bottom + n * line, ..r }
}

pub(super) fn lines(
    it: &Item,
    (x, y): (i32, i32),
    c: Cell,
    icon: i32,
    (spans, pre): (&[Span], &Prefix),
    s: impl Fn(i32) -> i32,
) -> Vec<(Vec<u16>, RECT, DRAW_TEXT_FORMAT)> {
    let Some(info) = it.info.as_deref() else { return vec![] };
    let name = cell::label_at((x, y), c, icon, &s);
    match c.mode {
        Mode::Details => spans[1..]
            .iter()
            .map(|&(col, a, b)| {
                let r = RECT { left: x + a + s(PAD), top: y, right: x + b - s(PAD), bottom: y + c.h };
                (field(info, col).to_vec(), r, if col == Col::Size { LINE | DT_RIGHT } else { LINE })
            })
            .collect(),
        Mode::Tiles => vec![(info.kind.clone(), below(name, 1, c.line), LINE), (info.bytes.clone(), below(name, 2, c.line), LINE)],
        Mode::Content => {
            let right = RECT { left: cell::split(x, c) + s(PAD), right: x + c.w - s(PAD), ..name };
            vec![
                (info.kind.clone(), below(name, 1, c.line), LINE),
                (labeled(&pre.modified, &info.modified), right, LINE),
                (labeled(&pre.size, &info.bytes), below(right, 1, c.line), LINE),
            ]
        }
        _ => vec![],
    }
}
