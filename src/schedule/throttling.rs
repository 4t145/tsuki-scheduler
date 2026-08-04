use chrono::TimeDelta;

pub use crate::Dtu;
use crate::{schedule::Schedule, timeset::pred_dtu};

/// A schedule that throttles the inner schedule by a given interval.
///
/// The interval is a **minimum** distance between two runs: after a run at `t`, every time point
/// of the inner schedule before `t + interval` is dropped, and a time point landing exactly on
/// `t + interval` is still allowed.
///
/// The emitted time points are always time points of the inner schedule, in particular
/// [`peek_next`](Schedule::peek_next) never invents a time point that
/// [`next`](Schedule::next) wouldn't return.
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
    /// The earliest time point allowed by the throttle.
    fn deadline(&self) -> Option<Dtu> {
        self.last_call.map(|last_call| last_call + self.interval)
    }
    /// Drop the time points of the inner schedule that are too close to the last call, they will
    /// never be emitted.
    fn skip_throttled(&mut self) -> Option<Dtu> {
        let Some(deadline) = self.deadline() else {
            return self.inner.peek_next();
        };
        let mut next = self.inner.peek_next()?;
        if next < deadline {
            // one jump: `forward_to` drops everything up to and including its argument
            self.inner
                .forward_to(pred_dtu(deadline).unwrap_or(deadline));
            next = self.inner.peek_next()?;
            // an inner schedule may not honor `forward_to`, drop the rest by hand
            while next < deadline {
                self.inner.next()?;
                next = self.inner.peek_next()?;
            }
        }
        Some(next)
    }
}

impl<S: Schedule> Schedule for Throttling<S> {
    fn peek_next(&mut self) -> Option<Dtu> {
        self.skip_throttled()
    }

    fn next(&mut self) -> Option<Dtu> {
        self.skip_throttled()?;
        self.inner.next().inspect(|this_call| {
            self.last_call = Some(*this_call);
        })
    }

    fn forward_to(&mut self, dtu: Dtu) {
        self.inner.forward_to(dtu);
    }
}
