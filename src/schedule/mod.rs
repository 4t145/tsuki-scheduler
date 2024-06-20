mod after;
pub use after::*;
mod or;
pub use or::*;
mod before;
pub use before::*;
mod then;
pub use then::*;
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
mod throttling;
pub use throttling::*;

pub trait Schedule {
    fn peek_next(&mut self) -> Option<Dtu>;
    fn next(&mut self) -> Option<Dtu>;
    fn forward_to(&mut self, dtu: Dtu);
}

impl<T> Schedule for T
where
    T: AsMut<dyn Schedule>,
{
    fn peek_next(&mut self) -> Option<Dtu> {
        self.as_mut().peek_next()
    }

    fn next(&mut self) -> Option<Dtu> {
        self.as_mut().next()
    }

    fn forward_to(&mut self, dtu: Dtu) {
        self.as_mut().forward_to(dtu)
    }
}

pub fn forward_to_default<S: Schedule>(schedule: &mut S, dtu: Dtu) {
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
    fn then<S: Schedule>(self, then: S) -> Then<Self, S> {
        then::Then::new(self, then)
    }
    fn throttling(self, interval: chrono::TimeDelta) -> Throttling<Self> {
        Throttling::new(self, interval)
    }
    fn dyn_box(self) -> Box<dyn Schedule>
    where
        Self: 'static,
    {
        Box::new(self)
    }
}

impl<S> ScheduleExt for S where S: Schedule + Sized {}
