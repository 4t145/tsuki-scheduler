use super::Schedule;
use crate::Dtu;

/// A wrapper around a schedule that only allows the task to run after a certain time.
#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq)]
pub struct After<S> {
    pub after: Dtu,
    pub inner: S,
}

impl<S: Schedule> After<S> {
    pub fn new(after: Dtu, mut inner: S) -> Self {
        inner.forward_to(after);
        Self { after, inner }
    }
    pub fn after(&self) -> Dtu {
        self.after
    }
}

impl<S: Schedule> Schedule for After<S> {
    fn peek_next(&mut self) -> Option<Dtu> {
        self.inner.peek_next()
    }

    fn next(&mut self) -> Option<Dtu> {
        self.inner.next()
    }

    fn forward_to(&mut self, dtu: Dtu) {
        self.inner.forward_to(dtu)
    }
}
