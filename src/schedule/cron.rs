use super::{IntoSchedule, Schedule};
use crate::Dtu;
use chrono::{DateTime, Local, Utc};
use cron::OwnedScheduleIterator;
use std::{iter::Peekable, str::FromStr};


/// A schedule that uses a cron expression to determine when to run a task.
pub struct Cron<Z: chrono::offset::TimeZone> {
    iterator: Peekable<OwnedScheduleIterator<Z>>,
    schedule: cron::Schedule,
    tz: Z,
}

impl<Z: chrono::offset::TimeZone> Cron<Z> {
    /// Create a new cron schedule from a cron expression and timezone.
    pub fn from_cron_schedule(schedule: cron::Schedule, timezone: Z) -> Self {
        Cron {
            schedule: schedule.clone(),
            iterator: OwnedScheduleIterator::new(schedule, Utc::now().with_timezone(&timezone))
                .peekable(),
            tz: timezone,
        }
    }
}

impl Cron<Utc> {
    /// Create a new cron schedule from a cron expression in UTC.
    pub fn utc_from_cron_expr(expr: &str) -> Result<Self, cron::error::Error> {
        let schedule = cron::Schedule::from_str(expr)?;
        Ok(Self::from_cron_schedule(schedule, Utc))
    }
}

impl Cron<Local> {
    /// Create a new cron schedule from a cron expression in the local timezone.
    pub fn local_from_cron_expr(expr: &str) -> Result<Self, cron::error::Error> {
        let schedule = cron::Schedule::from_str(expr)?;
        Ok(Self::from_cron_schedule(schedule, Local))
    }
}

impl<Z: chrono::offset::TimeZone> Schedule for Cron<Z> {
    fn peek_next(&mut self) -> Option<Dtu> {
        self.iterator.peek().map(DateTime::to_utc)
    }

    fn next(&mut self) -> Option<Dtu> {
        self.iterator.next().as_ref().map(DateTime::to_utc)
    }

    fn forward(&mut self, dtu: Dtu) {
        self.iterator =
            OwnedScheduleIterator::new(self.schedule.clone(), dtu.with_timezone(&self.tz))
                .peekable()
    }
}

impl IntoSchedule for cron::Schedule {
    type Output = Cron<chrono::Utc>;
    fn into_schedule(self) -> Self::Output {
        Cron {
            schedule: self.clone(),
            iterator: OwnedScheduleIterator::new(self, Utc::now()).peekable(),
            tz: Utc,
        }
    }
}
