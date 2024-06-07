use chrono::Utc;

use crate::{Dtu, Schedule};

pub struct Before<S> {
    before: Dtu,
    inner: S,
}

impl<S: Schedule> Before<S> {
    pub fn new(before: Dtu, inner: S) -> Self {
        Self { before, inner }
    }
    pub fn before(&self) -> Dtu {
        self.before
    }
}

impl<S: Schedule> Schedule for Before<S> {
    fn peek_next(&mut self) -> Option<Dtu> {
        let now = Utc::now();
        if now >= self.before {
            return None;
        }
        self.inner.peek_next()
    }

    fn next(&mut self) -> Option<Dtu> {
        let now = Utc::now();
        if now >= self.before {
            return None;
        }
        self.inner.next()
    }
}
