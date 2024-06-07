use std::iter::Peekable;

use crate::{Dtu, Schedule};

pub struct Iter<I: Iterator<Item = Dtu>> {
    inner: Peekable<I>,
}
impl<I: Iterator<Item = Dtu>> Iter<I> {
    pub fn new<It: IntoIterator<Item = Dtu, IntoIter = I>>(iter: It) -> Self {
        Self {
            inner: iter.into_iter().peekable(),
        }
    }
}
impl<I> Schedule for Iter<I>
where
    I: Iterator<Item = Dtu>,
{
    fn peek_next(&mut self) -> Option<crate::Dtu> {
        self.inner.peek().copied()
    }

    fn next(&mut self) -> Option<crate::Dtu> {
        self.inner.next()
    }
}
