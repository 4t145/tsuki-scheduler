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

pub trait Schedule: Send + 'static {
    fn peek_next(&mut self) -> Option<Dtu>;
    fn next(&mut self) -> Option<Dtu>;
    fn forward_to(&mut self, dtu: Dtu);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Never;

impl Schedule for Never {
    fn peek_next(&mut self) -> Option<Dtu> {
        None
    }

    fn next(&mut self) -> Option<Dtu> {
        None
    }

    fn forward_to(&mut self, _dtu: Dtu) {}
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
    fn builder(self) -> ScheduleBuilder<Self> {
        ScheduleBuilder::new(self)
    }
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
    fn dyn_box(self) -> Box<dyn Schedule> {
        Box::new(self)
    }
    fn box_or<S>(self, other: S) -> Box<dyn Schedule>
    where
        S: Schedule,
    {
        self.or(other).dyn_box()
    }
    fn box_after(self, time: crate::Dtu) -> Box<dyn Schedule> {
        self.after(time).dyn_box()
    }
    fn box_before(self, time: crate::Dtu) -> Box<dyn Schedule> {
        self.before(time).dyn_box()
    }
    fn box_then<S: Schedule>(self, then: S) -> Box<dyn Schedule> {
        self.then(then).dyn_box()
    }
    fn box_throttling(self, interval: chrono::TimeDelta) -> Box<dyn Schedule> {
        self.throttling(interval).dyn_box()
    }
}

impl<S> ScheduleExt for S where S: Schedule + Sized {}

pub struct ScheduleBuilder<S>
where
    S: Schedule,
{
    schedule: S,
}

impl Default for ScheduleBuilder<Never> {
    fn default() -> Self {
        Self { schedule: Never }
    }
}

impl<S0> ScheduleBuilder<S0>
where
    S0: Schedule,
{
    pub fn map<S1: Schedule>(self, map: impl FnOnce(S0) -> S1) -> ScheduleBuilder<S1> {
        ScheduleBuilder::new(map(self.schedule))
    }
    pub fn new(schedule: S0) -> Self {
        Self { schedule }
    }
    pub fn or<S1: Schedule>(self, other: S1) -> ScheduleBuilder<Or<S0, S1>> {
        self.map(|this| this.or(other))
    }
    pub fn after(self, time: crate::Dtu) -> ScheduleBuilder<After<S0>> {
        self.map(|this| this.after(time))
    }
    pub fn before(self, time: crate::Dtu) -> ScheduleBuilder<Before<S0>> {
        self.map(|this| this.before(time))
    }
    pub fn then<S1: Schedule>(self, then: S1) -> ScheduleBuilder<Then<S0, S1>> {
        self.map(|this| this.then(then))
    }
    pub fn throttling(self, interval: chrono::TimeDelta) -> ScheduleBuilder<Throttling<S0>> {
        self.map(|this| this.throttling(interval))
    }
    pub fn dyn_box(self) -> ScheduleBuilder<Box<dyn Schedule>> {
        self.map(|this| this.dyn_box())
    }
    pub fn build(self) -> S0 {
        self.schedule
    }
}
