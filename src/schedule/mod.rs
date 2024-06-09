use crate::Dtu;

mod after;
pub use after::*;
mod or;
pub use or::*;
mod before;
pub use before::*;
#[cfg(feature = "cron")]
mod cron;
#[cfg(feature = "cron")]
pub use cron::*;
mod iter;
pub use iter::*;
mod once;
pub use once::*;
mod period;
pub use period::*;

pub trait Schedule {
    fn peek_next(&mut self) -> Option<Dtu>;
    fn next(&mut self) -> Option<Dtu>;
    fn forward(&mut self, dtu: Dtu);
}

pub fn forward_default<S: Schedule>(schedule: &mut S, dtu: Dtu) {
    while let Some(next) = schedule.peek_next() {
        if next > dtu {
            break;
        }
        schedule.next();
    }
}

pub trait IntoSchedule {
    type Output: Schedule;
    fn into_schedule(self) -> Self::Output;
}

impl<S: Schedule> IntoSchedule for S {
    type Output = S;

    fn into_schedule(self) -> Self::Output {
        self
    }
}

/// shortcuts for creating combined schedules
pub trait ScheduleExt: Schedule + Sized {
    fn or<S: Schedule>(self, other: S) -> Or<Self, S> {
        or::Or::new(self, other)
    }
    fn after(self, time: crate::Dtu) -> After<Self> {
        after::After::new(time, self)
    }
    fn before(self, time: crate::Dtu) -> Before<Self> {
        before::Before::new(time, self)
    }
}

impl<S> ScheduleExt for S where S: Schedule + Sized {}
