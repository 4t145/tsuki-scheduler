use super::Schedule;
use crate::Dtu;

/// A wrapper around a schedule that only allows the task to run after a certain time.
pub struct After<S> {
    after: Dtu,
    inner: S,
}

impl<S: Schedule> After<S> {
    pub fn new(after: Dtu, mut inner: S) -> Self {
        inner.forward(after);
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

    fn forward(&mut self, dtu: Dtu) {
        self.inner.forward(dtu)
    }
}
