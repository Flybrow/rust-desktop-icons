pub const WAIT_MS: u64 = 2000;
pub const MAX_WAIT_MS: u64 = 30_000;
pub const RETRY_MS: u64 = 60_000;

pub fn wait_ms(paths: usize) -> u64 {
    (WAIT_MS + paths as u64 / 2).min(MAX_WAIT_MS)
}

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
    wait: u64,
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
            if passes == o.before && now < o.at + o.wait {
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
        let wait = wait_ms(self.load().max(keys.len()));
        self.out = Some(Out { keys, before, at: now, wait });
    }

    pub fn load(&self) -> usize {
        self.sent.iter().chain(self.out.iter().map(|o| &o.keys)).map(Vec::len).max().unwrap_or(0)
    }

    pub fn fail(&mut self, now: u64) {
        self.failed = Some(now);
        self.sent = None;
        self.out = None;
    }

    pub fn pending(&self) -> Option<(u32, u64)> {
        self.out.as_ref().map(|o| (o.before, o.wait))
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
        assert_eq!(f.pending(), Some((0, WAIT_MS)));
        assert_eq!(f.next(&a, 0, false, 100), Wait);
        assert_eq!(f.next(&keys(&["b"]), 0, true, 200), Wait, "never sends while a pass runs");
        assert_eq!(f.next(&a, 1, true, 300), Done);
        assert_eq!(f.pending(), None);
        assert_eq!(f.next(&a, 1, true, 400), Done, "an unchanged list is not sent again");
        assert_eq!(f.next(&keys(&["a", "b"]), 1, true, 500), Send);
    }

    #[test]
    fn big_lists_get_more_time_up_to_a_cap() {
        assert_eq!((wait_ms(0), wait_ms(4000), wait_ms(1_000_000)), (WAIT_MS, WAIT_MS + 2000, MAX_WAIT_MS));
        let mut f = Flight::default();
        let big: Vec<String> = (0..4000).map(|i| i.to_string()).collect();
        f.sending(big.clone(), 0, 0);
        assert_eq!(f.next(&big, 0, false, WAIT_MS + 1999), Wait, "a big first pass is not a failure");
        assert_eq!(f.next(&big, 1, true, WAIT_MS + 2000), Done);
        assert_eq!(f.load(), 4000);
        f.sending(vec![], 1, 0);
        assert_eq!(f.pending(), Some((1, WAIT_MS + 2000)), "showing everything again takes as long");
        assert_eq!(f.next(&[], 1, false, WAIT_MS + 2000), Broke);
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
