use super::Schedule;

#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq)]
pub struct Then<F, T> {
    firstly: F,
    then: T,
}

impl<F, T> Then<F, T> {
    pub fn new(firstly: F, then: T) -> Self {
        Self { firstly, then }
    }
}

impl<S0: Schedule, S1: Schedule> Schedule for Then<S0, S1> {
    fn peek_next(&mut self) -> Option<crate::Dtu> {
        if let Some(next) = self.firstly.peek_next() {
            self.then.forward_to(next);
            Some(next)
        } else {
            self.then.peek_next()
        }
    }

    fn next(&mut self) -> Option<crate::Dtu> {
        if let Some(next) = self.firstly.next() {
            self.then.forward_to(next);
            Some(next)
        } else {
            self.then.next()
        }
    }

    fn forward_to(&mut self, dtu: crate::Dtu) {
        self.firstly.forward_to(dtu);
        self.then.forward_to(dtu);
    }
}
