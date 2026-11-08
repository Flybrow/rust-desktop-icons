use super::Config;
use std::path::PathBuf;

pub const CAP: usize = 20;

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Moved(PathBuf, PathBuf),
    Created(PathBuf),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub before: Config,
    pub ops: Vec<Op>,
}

#[derive(Debug, Default)]
pub struct History {
    steps: Vec<Step>,
    open: Option<(Step, usize)>,
}

impl History {
    pub fn begin(&mut self, now: &Config) {
        match &mut self.open {
            Some((_, depth)) => *depth += 1,
            None => self.open = Some((Step { before: now.clone(), ops: vec![] }, 1)),
        }
    }

    pub fn note(&mut self, op: Op) {
        if let Some((step, _)) = &mut self.open {
            step.ops.push(op);
        }
    }

    pub fn end(&mut self, now: &Config) {
        let Some((step, depth)) = self.open.take() else { return };
        if depth > 1 {
            self.open = Some((step, depth - 1));
        } else if !step.ops.is_empty() || step.before != *now {
            self.steps.push(step);
            let extra = self.steps.len().saturating_sub(CAP);
            self.steps.drain(..extra);
        }
    }

    pub fn rebase(&mut self, now: &Config) {
        if let Some((step, _)) = &mut self.open {
            *step = Step { before: now.clone(), ops: vec![] };
        }
    }

    pub fn pop(&mut self) -> Option<Step> {
        self.steps.pop()
    }

    pub fn any(&self) -> bool {
        !self.steps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_keep_what_changed_and_nest() {
        let (a, b) = (Config::default(), Config { auto_sort: false, ..Config::default() });
        let mut h = History::default();
        h.begin(&a);
        h.end(&a);
        assert!(!h.any());
        h.begin(&a);
        h.begin(&a);
        h.note(Op::Moved("x".into(), "y".into()));
        h.end(&a);
        h.end(&b);
        let s = h.pop().unwrap();
        assert_eq!((s.before, s.ops), (a.clone(), vec![Op::Moved("x".into(), "y".into())]));
        h.note(Op::Created("z".into()));
        assert!(!h.any());
        for _ in 0..CAP + 5 {
            h.begin(&a);
            h.end(&b);
        }
        assert_eq!(h.steps.len(), CAP);
        let mut h = History::default();
        h.begin(&a);
        h.note(Op::Created("z".into()));
        h.rebase(&b);
        h.end(&b);
        assert!(!h.any());
    }

    #[test]
    fn oldest_steps_go_first_and_undo_is_last_in_first_out() {
        let cfg = |roll_ms| Config { roll_ms, ..Config::default() };
        let mut h = History::default();
        h.end(&cfg(1));
        h.rebase(&cfg(1));
        assert!(!h.any());
        for n in 0..CAP as u32 + 3 {
            h.begin(&cfg(n));
            h.end(&cfg(n + 1));
        }
        let befores: Vec<u32> = std::iter::from_fn(|| h.pop()).map(|s| s.before.roll_ms).collect();
        assert_eq!(befores, (3..CAP as u32 + 3).rev().collect::<Vec<_>>());
        h.begin(&cfg(1));
        h.rebase(&cfg(5));
        h.note(Op::Created("a".into()));
        h.end(&cfg(5));
        assert_eq!(h.pop().map(|s| (s.before.roll_ms, s.ops.len())), Some((5, 1)));
    }
}
