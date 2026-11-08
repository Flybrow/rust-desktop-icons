use super::{Config, Fence, Look};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Field {
    Color,
    Alpha,
    Icon,
    Tint,
    Chameleon,
    AutoHeight,
    Labels,
    Align,
    LabelSize,
    LabelColor,
    Spacing,
    Browse,
    TitleAlign,
    TitleSize,
    BarColor,
    BarHover,
    Wallpaper,
    Thumbs,
}

impl Field {
    pub const ALL: [Field; 18] = [
        Field::Color,
        Field::Alpha,
        Field::Icon,
        Field::Tint,
        Field::Chameleon,
        Field::AutoHeight,
        Field::Labels,
        Field::Align,
        Field::LabelSize,
        Field::LabelColor,
        Field::Spacing,
        Field::Browse,
        Field::TitleAlign,
        Field::TitleSize,
        Field::BarColor,
        Field::BarHover,
        Field::Wallpaper,
        Field::Thumbs,
    ];

    fn copy(self, from: &Look, to: &mut Look) {
        match self {
            Field::Color => to.color = from.color,
            Field::Alpha => to.alpha = from.alpha,
            Field::Icon => to.icon = from.icon,
            Field::Tint => to.tint = from.tint,
            Field::Chameleon => to.chameleon = from.chameleon,
            Field::AutoHeight => to.auto_height = from.auto_height,
            Field::Labels => to.labels = from.labels,
            Field::Align => to.align = from.align,
            Field::LabelSize => to.label_size = from.label_size,
            Field::LabelColor => to.label_color = from.label_color,
            Field::Spacing => to.spacing = from.spacing,
            Field::Browse => to.browse = from.browse,
            Field::TitleAlign => to.title_align = from.title_align,
            Field::TitleSize => to.title_size = from.title_size,
            Field::BarColor => to.bar_color = from.bar_color,
            Field::BarHover => to.bar_hover = from.bar_hover,
            Field::Wallpaper => to.wallpaper = from.wallpaper,
            Field::Thumbs => to.thumbs = from.thumbs,
        }
    }

    fn differs(self, a: &Look, b: &Look) -> bool {
        let mut c = b.clone();
        self.copy(a, &mut c);
        c != *b
    }
}

impl Fence {
    pub fn set(&mut self, field: Field, edit: impl FnOnce(&mut Look)) {
        edit(&mut self.look);
        let custom = self.custom.get_or_insert_with(Vec::new);
        if !custom.contains(&field) {
            custom.push(field);
        }
    }

    pub fn customized(&self) -> bool {
        self.custom.as_ref().is_some_and(|c| !c.is_empty())
    }
}

impl Config {
    pub fn inherit(&mut self) {
        let base = self.look.clone();
        for f in &mut self.fences {
            let custom = f.custom.take().unwrap_or_else(|| Field::ALL.into_iter().filter(|x| x.differs(&f.look, &base)).collect());
            for field in Field::ALL.into_iter().filter(|x| !custom.contains(x)) {
                field.copy(&base, &mut f.look);
            }
            f.custom = Some(custom);
        }
    }

    pub fn reset_look(&mut self, fence: Option<u64>) {
        for f in self.fences.iter_mut().filter(|f| fence.is_none_or(|id| f.id == id)) {
            f.custom = Some(vec![]);
        }
        self.inherit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fence(id: u64, look: Look) -> Fence {
        Fence { id, look, ..Fence::default() }
    }

    #[test]
    fn fences_follow_defaults_except_their_own_changes() {
        let base = Look::default();
        let red = Look { color: 0xFF, ..base.clone() };
        let mut c = Config { look: base.clone(), fences: vec![fence(1, base.clone()), fence(2, red)], ..Config::default() };
        c.inherit();
        assert_eq!((c.fences[0].custom.clone(), c.fences[1].custom.clone()), (Some(vec![]), Some(vec![Field::Color])));
        c.look.color = 0xAA;
        c.look.alpha = 100;
        c.inherit();
        assert_eq!((c.fences[0].look.color, c.fences[1].look.color), (0xAA, 0xFF));
        assert_eq!((c.fences[0].look.alpha, c.fences[1].look.alpha), (100, 100));
        c.fences[0].set(Field::Labels, |l| l.labels = false);
        c.look.labels = true;
        c.inherit();
        assert!(!c.fences[0].look.labels && c.fences[0].customized() && c.fences[1].look.labels);
        c.reset_look(Some(1));
        assert!(c.fences[0].look.labels && !c.fences[0].customized() && c.fences[1].customized());
        c.reset_look(None);
        assert_eq!(c.fences[1].look.color, 0xAA);
        let back: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back.fences[1].custom, Some(vec![]));
    }

    #[test]
    fn every_field_is_inherited_and_can_be_overridden() {
        use crate::domain::{
            grid::Align,
            icons::{LabelSize, Spacing},
        };
        let other = Look {
            color: 0x123456,
            alpha: 77,
            icon: 96,
            tint: Some(0xFF),
            chameleon: true,
            auto_height: false,
            labels: false,
            align: Align::Right,
            label_size: LabelSize::Large,
            label_color: Some(0xAB),
            spacing: Spacing::Wide,
            browse: false,
            title_align: Align::Right,
            title_size: LabelSize::Small,
            bar_color: Some(0xCD),
            bar_hover: true,
            wallpaper: true,
            thumbs: false,
        };
        let base = Look::default();
        assert!(Field::ALL.iter().all(|f| f.differs(&other, &base)), "a field is not copied by Field::copy");
        let mut c = Config { look: base.clone(), fences: vec![fence(1, base.clone())], ..Config::default() };
        c.inherit();
        c.look = other.clone();
        c.inherit();
        assert_eq!(c.fences[0].look, other);
        for f in Field::ALL {
            let mut c = Config { look: base.clone(), fences: vec![fence(1, base.clone())], ..Config::default() };
            c.inherit();
            c.fences[0].set(f, |l| f.copy(&other, l));
            c.look = Look { alpha: 1, ..base.clone() };
            c.inherit();
            let mut expected = Look { alpha: 1, ..base.clone() };
            f.copy(&other, &mut expected);
            assert_eq!(c.fences[0].look, expected, "{f:?}");
        }
    }
}
