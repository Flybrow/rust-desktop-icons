use super::paint::measure;
use crate::{
    layered::{glyph as g, line_width},
    update::notes::Block,
};
use windows::Win32::Graphics::Gdi::HFONT;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Ink {
    Text,
    Accent,
    Dim,
}

#[derive(Clone, Copy)]
pub(super) struct Fonts {
    pub section: HFONT,
    pub title: HFONT,
    pub desc: HFONT,
    pub icon: HFONT,
}

pub(super) type Rect = (i32, i32, i32, i32);

pub(super) struct Line {
    pub ink: Ink,
    pub font: HFONT,
    pub text: String,
    pub rect: Rect,
}

#[derive(Default)]
pub(super) struct Placed {
    pub lines: Vec<Line>,
    pub cards: Vec<Rect>,
    pub height: i32,
}

impl Placed {
    pub fn shift(&mut self, dy: i32) {
        self.lines.iter_mut().for_each(|l| (l.rect.1, l.rect.3) = (l.rect.1 + dy, l.rect.3 + dy));
        self.cards.iter_mut().for_each(|c| (c.1, c.3) = (c.1 + dy, c.3 + dy));
        self.height += dy;
    }

    fn put(&mut self, ink: Ink, font: HFONT, text: &str, (x0, x1): (i32, i32), y: i32) -> i32 {
        let h = measure(font, text, x1 - x0);
        self.lines.push(Line { ink, font, text: text.to_string(), rect: (x0, y, x1, y + h) });
        h
    }
}

const GLYPHS: [char; 3] = [g::STAR, g::BOLT, g::REPAIR];

pub(super) fn place(blocks: &[Block], f: Fonts, width: i32, s: impl Fn(i32) -> i32) -> Placed {
    let (mut p, mut y, mut section, (px, py)) = (Placed::default(), 0, 0, (s(12), s(10)));
    for (i, b) in blocks.iter().enumerate() {
        match b {
            Block::Section(t) => {
                y += if i > 0 { s(12) } else { 0 };
                let n = blocks[i + 1..].iter().take_while(|b| matches!(b, Block::Item(..))).count();
                p.put(Ink::Accent, f.icon, &GLYPHS[section.min(2)].to_string(), (0, s(20)), y + s(2));
                section += 1;
                let label = s(24) + line_width(f.section, &t.encode_utf16().collect::<Vec<_>>());
                let h = p.put(Ink::Accent, f.section, t, (s(24), width), y);
                p.put(Ink::Dim, f.section, &format!("· {n}"), (label + s(6), width), y);
                y += h + s(8);
            }
            Block::Item(title, desc) => {
                let top = y;
                y += py + p.put(Ink::Text, f.title, title, (px, width - px), y + py);
                if !desc.is_empty() {
                    y += s(2) + p.put(Ink::Dim, f.desc, desc, (px, width - px), y + s(2));
                }
                y += py;
                p.cards.push((0, top, width, y));
                y += s(6);
            }
        }
    }
    p.height = y;
    p
}
