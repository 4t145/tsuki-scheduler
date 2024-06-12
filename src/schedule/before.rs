use super::Schedule;
use crate::Dtu;

/// A wrapper around a schedule that only allows the task to run before a certain time.
#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq)]
pub struct Before<S> {
    pub before: Dtu,
    pub inner: S,
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
        let next = self.inner.peek_next()?;
        if next >= self.before {
            None
        } else {
            Some(next)
        }
    }

    fn next(&mut self) -> Option<Dtu> {
        self.peek_next()?;
        self.inner.next()
    }

    fn forward_to(&mut self, dtu: Dtu) {
        self.inner.forward_to(dtu)
    }
}
