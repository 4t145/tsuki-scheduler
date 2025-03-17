use chrono::TimeDelta;

pub use crate::Dtu;
use crate::schedule::Schedule;

/// A schedule that throttles the inner schedule by a given interval.
pub struct Throttling<S> {
    pub inner: S,
    pub last_call: Option<Dtu>,
    pub interval: TimeDelta,
}

impl<S: Schedule> Throttling<S> {
    pub fn new(inner: S, interval: TimeDelta) -> Self {
        Self {
            inner,
            last_call: None,
            interval,
        }
    }
}

impl<S: Schedule> Schedule for Throttling<S> {
    fn peek_next(&mut self) -> Option<Dtu> {
        let next = self.inner.peek_next()?;
        if let Some(last_call) = self.last_call {
            if next < last_call + self.interval {
                return Some(last_call + self.interval);
            }
        }
        Some(next)
    }

    fn next(&mut self) -> Option<Dtu> {
        if let Some(last_call) = self.last_call {
            self.forward_to(last_call + self.interval);
        }
        self.inner.next().inspect(|this_call| {
            self.last_call = Some(*this_call);
        })
    }

    fn forward_to(&mut self, dtu: Dtu) {
        self.inner.forward_to(dtu);
    }
}
