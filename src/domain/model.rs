use super::{Look, compat, inherit::Field, kind::Kind, matching::Expire, order::Sort, pins::Pin, system::Sys};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(super) fn clean(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).take(64).collect()
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tab {
    pub id: u64,
    pub title: String,
    pub portal: Option<PathBuf>,
    #[serde(deserialize_with = "compat::known")]
    pub kinds: Vec<Kind>,
    #[serde(deserialize_with = "compat::or_default")]
    pub sort: Sort,
    pub order: Vec<String>,
    #[serde(skip_serializing)]
    pub recycle: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", deserialize_with = "compat::known")]
    pub system: Vec<Sys>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pins: Vec<Pin>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire: Option<Expire>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub filter: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fence {
    pub id: u64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub rolled: bool,
    pub locked: bool,
    pub vertical: bool,
    #[serde(flatten)]
    pub look: Look,
    pub tabs: Vec<Tab>,
    pub tab: usize,
    #[serde(default)]
    pub custom: Option<Vec<Field>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desktop: Option<String>,
}

impl Default for Fence {
    fn default() -> Self {
        Self {
            id: 0,
            title: String::new(),
            x: 100,
            y: 100,
            w: 360,
            h: 240,
            rolled: false,
            locked: false,
            vertical: false,
            look: Look::default(),
            tabs: vec![],
            tab: 0,
            custom: None,
            group: None,
            desktop: None,
        }
    }
}

impl Fence {
    pub const MIN: (i32, i32) = (120, 80);

    pub fn new(id: u64, title: &str, (x, y, w, h): (i32, i32, i32, i32), look: Look) -> Self {
        let tabs = vec![Tab { id, title: title.into(), ..Tab::default() }];
        Self { id, x, y, w, h, look, tabs, custom: Some(vec![]), ..Self::default() }
    }

    pub fn active(&self) -> &Tab {
        &self.tabs[self.tab.min(self.tabs.len() - 1)]
    }

    pub fn active_mut(&mut self) -> &mut Tab {
        let i = self.tab.min(self.tabs.len() - 1);
        &mut self.tabs[i]
    }

    pub(super) fn sanitized(mut self) -> Self {
        self.w = self.w.clamp(Self::MIN.0, 10_000);
        self.h = self.h.clamp(Self::MIN.1, 10_000);
        self.x = self.x.clamp(-20_000, 20_000);
        self.y = self.y.clamp(-20_000, 20_000);
        self.look = self.look.sanitized();
        if self.tabs.is_empty() {
            self.tabs.push(Tab { id: self.id, title: std::mem::take(&mut self.title), ..Tab::default() });
        }
        self.title.clear();
        for t in &mut self.tabs {
            t.title = clean(&t.title);
            t.kinds.dedup();
        }
        self.tab = self.tab.min(self.tabs.len() - 1);
        self
    }
}
