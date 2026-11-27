use crate::{
    domain::{Fence, side::Side},
    layered::Frame,
    shell,
};
use std::{collections::HashSet, path::PathBuf};
use windows::Win32::{Foundation::HWND, UI::Shell::IContextMenu};

#[derive(Clone, PartialEq)]
pub struct Item {
    pub path: PathBuf,
    pub name: Vec<u16>,
    pub icon: i32,
    pub thumb: Option<crate::shell::thumb::Thumb>,
    pub info: Option<Box<crate::shell::details::Info>>,
}

pub struct Filter {
    pub query: String,
    pub all: Vec<Item>,
    pub caret: bool,
}

pub enum Rename {
    Title(u64),
    Item(PathBuf),
}

pub struct View {
    pub hwnd: HWND,
    pub id: u64,
    pub items: Vec<Item>,
    pub shown: Option<(u64, Option<PathBuf>)>,
    pub scroll: i32,
    pub edit: Option<(HWND, Rename)>,
    pub native: Option<IContextMenu>,
    pub hover: Option<usize>,
    pub inside: bool,
    pub hold: bool,
    pub unroll: f32,
    pub floor: i32,
    pub opening: bool,
    pub side: Side,
    pub glow: f32,
    pub tick: Option<f64>,
    pub frame: Option<Frame>,
    pub grow: Option<(i32, f32)>,
    pub labels: Option<crate::layered::Labels>,
    pub wall: Option<((i32, i32, i32, i32), u32)>,
    pub frost: Option<crate::render::frost::Cached>,
    pub filter: Option<Filter>,
    pub cursor: Option<usize>,
    pub sizing: bool,
    pub busy: bool,
    pub drag: Option<([i32; 4], (i32, i32))>,
    pub watches: Vec<shell::Watch>,
    pub cache: Vec<(u64, Vec<Item>)>,
    pub gens: Vec<(u64, u64)>,
    pub dirty: Vec<usize>,
    pub loading: bool,
    pub queued: Vec<crate::domain::Tab>,
    pub browse: Vec<PathBuf>,
    pub backed: bool,
    pub bar_tab: Option<usize>,
    pub selected: HashSet<PathBuf>,
    pub band: Option<((i32, i32), (i32, i32))>,
    pub stale: bool,
    pub resize: Option<(usize, Vec<crate::domain::columns::Column>, Vec<crate::domain::columns::Span>)>,
}

impl View {
    pub fn all(&self) -> &[Item] {
        self.filter.as_ref().map_or(&self.items, |f| &f.all)
    }

    pub(super) fn new(hwnd: HWND, f: &Fence) -> Self {
        let unroll = if f.rolled { 0. } else { 1. };
        let glow = if f.look.chameleon { 0. } else { 1. };
        Self {
            hwnd,
            id: f.id,
            items: vec![],
            shown: None,
            scroll: 0,
            edit: None,
            native: None,
            hover: None,
            inside: false,
            hold: false,
            floor: 0,
            unroll,
            opening: false,
            side: Side::default(),
            glow,
            tick: None,
            frame: None,
            grow: None,
            labels: None,
            wall: None,
            frost: None,
            filter: None,
            cursor: None,
            sizing: false,
            busy: false,
            drag: None,
            watches: vec![],
            cache: vec![],
            gens: vec![],
            dirty: vec![],
            loading: false,
            queued: vec![],
            browse: vec![],
            backed: false,
            bar_tab: None,
            selected: HashSet::new(),
            band: None,
            resize: None,
            stale: false,
        }
    }
}
