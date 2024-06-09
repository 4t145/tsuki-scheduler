use std::iter::Peekable;

use super::{IntoSchedule, Schedule};
use crate::Dtu;

/// A schedule that iterates over a sorted list of `Dtu`s.
///
/// # Warning
/// please ensure that the list of `Dtu`s is sorted.
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

    fn forward(&mut self, dtu: Dtu) {
        super::forward_default(self, dtu)
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

impl IntoSchedule for Option<Dtu> {
    type Output = Iter<std::option::IntoIter<Dtu>>;

    fn into_schedule(self) -> Self::Output {
        Iter::new(self)
    }
}
