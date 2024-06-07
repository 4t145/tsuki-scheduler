use crate::Schedule;

pub mod after;
pub mod and;
pub mod before;
#[cfg(feature = "cron")]
pub mod cron;
pub mod iter;
pub mod once;
pub mod period;

pub trait ScheduleExt: Schedule + Sized {
    fn and<S: Schedule>(self, other: S) -> and::And<Self, S> {
        and::And::new(self, other)
    }
    fn after(self, time: crate::Dtu) -> after::After<Self> {
        after::After::new(time, self)
    }
    fn before(self, time: crate::Dtu) -> before::Before<Self> {
        before::Before::new(time, self)
    }
}

impl<S> ScheduleExt for S where S: Schedule + Sized {}
