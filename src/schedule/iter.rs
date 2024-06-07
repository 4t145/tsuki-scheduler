use std::iter::Peekable;

use crate::{Dtu, IntoSchedule, Schedule};

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

impl<const N: usize> IntoSchedule for [Dtu; N] {
    type Output = Iter<std::array::IntoIter<Dtu, N>>;

    fn into_schedule(mut self) -> Self::Output {
        self.sort();
        Iter::new(self)
    }
}

impl IntoSchedule for Vec<Dtu> {
    type Output = Iter<std::vec::IntoIter<Dtu>>;

    fn into_schedule(mut self) -> Self::Output {
        self.sort();
        Iter::new(self)
    }
}
