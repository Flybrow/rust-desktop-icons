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
}

impl Field {
    pub const ALL: [Field; 8] =
        [Field::Color, Field::Alpha, Field::Icon, Field::Tint, Field::Chameleon, Field::AutoHeight, Field::Labels, Field::Align];

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
}
