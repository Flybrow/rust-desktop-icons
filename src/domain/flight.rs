pub const WAIT_MS: u64 = 2000;
pub const RETRY_MS: u64 = 60_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Next {
    Send,
    Wait,
    Done,
    Failed,
    Broke,
}

struct Out {
    keys: Vec<String>,
    before: u32,
    at: u64,
}

#[derive(Default)]
pub struct Flight {
    sent: Option<Vec<String>>,
    out: Option<Out>,
    failed: Option<u64>,
}

impl Flight {
    pub fn next(&mut self, keys: &[String], passes: u32, done: bool, now: u64) -> Next {
        if let Some(o) = self.out.take() {
            if passes == o.before && now < o.at + WAIT_MS {
                self.out = Some(o);
                return Next::Wait;
            }
            if passes == o.before || !done {
                self.fail(now);
                return Next::Broke;
            }
            self.sent = Some(o.keys);
            self.failed = None;
        }
        if self.failed.is_some_and(|t| now < t + RETRY_MS) {
            Next::Failed
        } else if self.sent.as_deref() == Some(keys) {
            Next::Done
        } else {
            Next::Send
        }
    }

    pub fn sending(&mut self, keys: Vec<String>, before: u32, now: u64) {
        self.out = Some(Out { keys, before, at: now });
    }

    pub fn fail(&mut self, now: u64) {
        self.failed = Some(now);
        self.sent = None;
        self.out = None;
    }

    pub fn pending(&self) -> Option<u32> {
        self.out.as_ref().map(|o| o.before)
    }
}

#[cfg(test)]
mod tests {
    use super::{Next::*, *};

    fn keys(k: &[&str]) -> Vec<String> {
        k.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sends_once_then_waits_for_the_answer() {
        let mut f = Flight::default();
        let a = keys(&["a"]);
        assert_eq!(f.next(&a, 0, false, 0), Send);
        f.sending(a.clone(), 0, 0);
        assert_eq!(f.pending(), Some(0));
        assert_eq!(f.next(&a, 0, false, 100), Wait);
        assert_eq!(f.next(&keys(&["b"]), 0, true, 200), Wait, "never sends while a pass runs");
        assert_eq!(f.next(&a, 1, true, 300), Done);
        assert_eq!(f.pending(), None);
        assert_eq!(f.next(&a, 1, true, 400), Done, "an unchanged list is not sent again");
        assert_eq!(f.next(&keys(&["a", "b"]), 1, true, 500), Send);
    }

    #[test]
    fn a_failed_or_silent_pass_falls_back_then_retries() {
        let mut f = Flight::default();
        let a = keys(&["a"]);
        f.sending(a.clone(), 4, 0);
        assert_eq!(f.next(&a, 5, false, 10), Broke, "the pass stopped before the end");
        assert_eq!(f.next(&a, 5, false, 20), Failed);
        assert_eq!(f.next(&a, 5, false, RETRY_MS - 1), Failed);
        assert_eq!(f.next(&a, 5, false, RETRY_MS + 10), Send);
        f.sending(a.clone(), 5, RETRY_MS + 10);
        assert_eq!(f.next(&a, 5, true, RETRY_MS + 10 + WAIT_MS - 1), Wait, "a stale DONE is not an answer");
        assert_eq!(f.next(&a, 5, true, RETRY_MS + 10 + WAIT_MS), Broke, "no answer in time");
        let t = RETRY_MS * 3;
        f.sending(a.clone(), 5, t);
        assert_eq!(f.next(&a, 6, true, t + 5), Done, "a success clears the failure");
        f.fail(t + 6);
        assert_eq!(f.pending(), None);
        assert_eq!(f.next(&a, 6, true, t + 6 + RETRY_MS), Send, "a failure forgets what was sent");
    }
}
