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
mod never;
pub use never::*;

pub trait Schedule: Send + 'static {
    fn peek_next(&mut self) -> Option<Dtu>;
    fn next(&mut self) -> Option<Dtu>;
    fn forward_to(&mut self, dtu: Dtu);
}

impl<T> Schedule for T
where
    T: AsMut<dyn Schedule>,
    T: Send + 'static,
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
    fn dyn_builder(self) -> ScheduleDynBuilder {
        ScheduleDynBuilder::new(self)
    }
    fn or<S: IntoSchedule>(self, other: S) -> Or<Self, S::Output> {
        or::Or::new(self, other.into_schedule())
    }
    fn after(self, time: crate::Dtu) -> After<Self> {
        after::After::new(time, self)
    }
    fn before(self, time: crate::Dtu) -> Before<Self> {
        before::Before::new(time, self)
    }
    fn then<S: IntoSchedule>(self, then: S) -> Then<Self, S::Output> {
        then::Then::new(self, then.into_schedule())
    }
    fn throttling(self, interval: chrono::TimeDelta) -> Throttling<Self> {
        Throttling::new(self, interval)
    }
    fn dyn_box(self) -> Box<dyn Schedule> {
        Box::new(self)
    }
}

impl<S> ScheduleExt for S where S: Schedule + Sized {}

/// Dynamic builder api for creating combined schedules
pub struct ScheduleDynBuilder {
    schedule: Box<dyn Schedule>,
}

impl Default for ScheduleDynBuilder {
    fn default() -> Self {
        Self {
            schedule: Never.dyn_box(),
        }
    }
}

impl ScheduleDynBuilder {
    pub fn map<S: Schedule>(self, map: impl FnOnce(Box<dyn Schedule>) -> S) -> Self {
        ScheduleDynBuilder::new(map(self.schedule))
    }
    pub fn new<S: IntoSchedule>(schedule: S) -> Self {
        Self {
            schedule: schedule.into_schedule().dyn_box(),
        }
    }
    pub fn or<S: IntoSchedule>(self, other: S) -> ScheduleDynBuilder {
        self.map(|this| this.or(other))
    }
    pub fn after(self, time: crate::Dtu) -> ScheduleDynBuilder {
        self.map(|this| this.after(time))
    }
    pub fn before(self, time: crate::Dtu) -> ScheduleDynBuilder {
        self.map(|this| this.before(time))
    }
    pub fn then<S: IntoSchedule>(self, then: S) -> ScheduleDynBuilder {
        self.map(|this| this.then(then))
    }
    pub fn throttling(self, interval: chrono::TimeDelta) -> ScheduleDynBuilder {
        self.map(|this| this.throttling(interval))
    }
    pub fn build(self) -> Box<dyn Schedule> {
        self.schedule
    }
}

impl IntoSchedule for ScheduleDynBuilder {
    type Output = Box<dyn Schedule>;
    fn into_schedule(self) -> Self::Output {
        self.build()
    }
}
