use crate::{Dtu, Schedule};

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
}
