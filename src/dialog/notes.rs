use super::paint::measure;
use crate::update::notes::Block;
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
}

pub(super) struct Line {
    pub ink: Ink,
    pub font: HFONT,
    pub text: String,
    pub rect: (i32, i32, i32, i32),
}

pub(super) fn place(blocks: &[Block], f: Fonts, width: i32, s: impl Fn(i32) -> i32) -> (Vec<Line>, i32) {
    let (mut y, mut out, indent) = (0, vec![], s(16));
    let mut line = |ink, font, text: &str, (x0, x1): (i32, i32), y: i32| {
        let h = measure(font, text, x1 - x0);
        out.push(Line { ink, font, text: text.to_string(), rect: (x0, y, x1, y + h) });
        h
    };
    for (i, b) in blocks.iter().enumerate() {
        match b {
            Block::Section(t) => {
                y += if i > 0 { s(14) } else { 0 };
                y += line(Ink::Accent, f.section, t, (0, width), y) + s(6);
            }
            Block::Item(title, desc) => {
                line(Ink::Accent, f.title, "•", (s(2), indent), y);
                y += line(Ink::Text, f.title, title, (indent, width), y);
                if !desc.is_empty() {
                    y += line(Ink::Dim, f.desc, desc, (indent, width), y + s(1)) + s(1);
                }
                y += s(8);
            }
        }
    }
    (out, y)
}
