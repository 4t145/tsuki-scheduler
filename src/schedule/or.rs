use super::Schedule;
use crate::Dtu;


/// Combines two schedules into one that runs when one of the schedules is ready.
pub struct Or<S0, S1>(pub S0, pub S1);
impl<S0, S1> Schedule for Or<S0, S1>
where
    S0: Schedule,
    S1: Schedule,
{
    fn peek_next(&mut self) -> Option<Dtu> {
        match (self.0.peek_next(), self.1.peek_next()) {
            (None, None) => None,
            (None, Some(next)) => Some(next),
            (Some(next), None) => Some(next),
            (Some(next_0), Some(next_1)) => Some(next_0.min(next_1)),
        }
    }
    fn next(&mut self) -> Option<Dtu> {
        match (self.0.peek_next(), self.1.peek_next()) {
            (None, None) => None,
            (None, Some(next)) => {
                self.1.next();
                Some(next)
            }
            (Some(next), None) => {
                self.0.next();
                Some(next)
            }
            (Some(next_0), Some(next_1)) => {
                if next_0 < next_1 {
                    self.0.next();
                    Some(next_0)
                } else {
                    self.1.next();
                    Some(next_1)
                }
            }
        }
    }
    fn forward(&mut self, dtu: Dtu) {
        self.0.forward(dtu);
        self.1.forward(dtu);
    }
}

impl<S0, S1> Or<S0, S1> {
    #[inline]
    pub fn new(s0: S0, s1: S1) -> Self {
        Self(s0, s1)
    }
}
