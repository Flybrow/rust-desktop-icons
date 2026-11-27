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
    pub fn begin(&mut self, now: impl FnOnce() -> Option<Config>) -> bool {
        match &mut self.open {
            Some((_, depth)) => *depth += 1,
            None => match now() {
                Some(before) => self.open = Some((Step { before, ops: vec![] }, 1)),
                None => return false,
            },
        }
        true
    }

    pub fn note(&mut self, op: Op) {
        if let Some((step, _)) = &mut self.open {
            step.ops.push(op);
        }
    }

    pub fn end(&mut self, now: impl FnOnce() -> Option<Config>) {
        let Some((step, depth)) = self.open.take() else { return };
        if depth > 1 {
            self.open = Some((step, depth - 1));
        } else if !step.ops.is_empty() || now().as_ref() != Some(&step.before) {
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

    pub fn clear(&mut self, now: Option<&Config>) {
        self.steps.clear();
        match now {
            Some(c) => self.rebase(c),
            None => self.open = None,
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
    fn a_step_needs_a_real_snapshot() {
        let mut h = History::default();
        assert!(!h.begin(|| None));
        h.note(Op::Created("z".into()));
        h.end(|| Some(Config::default()));
        assert!(!h.any());
        assert!(h.begin(|| Some(Config::default())) && h.begin(|| None));
        h.end(|| None);
        h.end(|| None);
        assert!(h.pop().is_some_and(|s| s.before == Config::default() && s.ops.is_empty()));
    }

    #[test]
    fn steps_keep_what_changed_and_nest() {
        let (a, b) = (Config::default(), Config { auto_sort: false, ..Config::default() });
        let mut h = History::default();
        h.begin(|| Some(a.clone()));
        h.end(|| Some(a.clone()));
        assert!(!h.any());
        h.begin(|| Some(a.clone()));
        h.begin(|| Some(a.clone()));
        h.note(Op::Moved("x".into(), "y".into()));
        h.end(|| Some(a.clone()));
        h.end(|| Some(b.clone()));
        let s = h.pop().unwrap();
        assert_eq!((s.before, s.ops), (a.clone(), vec![Op::Moved("x".into(), "y".into())]));
        h.note(Op::Created("z".into()));
        assert!(!h.any());
        for _ in 0..CAP + 5 {
            h.begin(|| Some(a.clone()));
            h.end(|| Some(b.clone()));
        }
        assert_eq!(h.steps.len(), CAP);
        let mut h = History::default();
        h.begin(|| Some(a.clone()));
        h.note(Op::Created("z".into()));
        h.rebase(&b);
        h.end(|| Some(b.clone()));
        assert!(!h.any());
    }

    #[test]
    fn oldest_steps_go_first_and_undo_is_last_in_first_out() {
        let cfg = |roll_ms| Config { roll_ms, ..Config::default() };
        let mut h = History::default();
        h.end(|| Some(cfg(1)));
        h.rebase(&cfg(1));
        assert!(!h.any());
        for n in 0..CAP as u32 + 3 {
            h.begin(|| Some(cfg(n)));
            h.end(|| Some(cfg(n + 1)));
        }
        let befores: Vec<u32> = std::iter::from_fn(|| h.pop()).map(|s| s.before.roll_ms).collect();
        assert_eq!(befores, (3..CAP as u32 + 3).rev().collect::<Vec<_>>());
        h.begin(|| Some(cfg(1)));
        h.rebase(&cfg(5));
        h.note(Op::Created("a".into()));
        h.end(|| Some(cfg(5)));
        assert_eq!(h.pop().map(|s| (s.before.roll_ms, s.ops.len())), Some((5, 1)));
    }

    #[test]
    fn clear_forgets_every_step_and_the_open_one() {
        let (a, b) = (Config::default(), Config { auto_sort: false, ..Config::default() });
        let mut h = History::default();
        h.begin(|| Some(a.clone()));
        h.end(|| Some(b.clone()));
        h.begin(|| Some(b.clone()));
        h.note(Op::Created("z".into()));
        h.clear(Some(&a));
        h.end(|| Some(a.clone()));
        assert!(!h.any());
        h.begin(|| Some(b.clone()));
        h.clear(None);
        h.end(|| Some(a.clone()));
        assert!(!h.any(), "a clear without a snapshot drops the open step");
        h.begin(|| Some(a.clone()));
        h.end(|| Some(b.clone()));
        assert!(h.any(), "steps after a clear are recorded");
    }

    #[test]
    fn nested_steps_close_with_the_outermost_one() {
        let (c, mut h) = (Config::default(), History::default());
        assert!(!h.any());
        h.begin(|| Some(c.clone()));
        h.begin(|| Some(c.clone()));
        h.note(Op::Created("a".into()));
        h.end(|| Some(c.clone()));
        assert!(!h.any());
        h.note(Op::Created("b".into()));
        h.end(|| Some(c.clone()));
        assert!(h.any());
        assert_eq!(h.pop().unwrap().ops.len(), 2);
        h.end(|| Some(c.clone()));
        assert!(!h.any());
    }
}
