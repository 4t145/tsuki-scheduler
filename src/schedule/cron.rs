use std::{iter::Peekable, str::FromStr};

use chrono::{Local, Utc};
use cron::OwnedScheduleIterator;

use crate::{Dtu, IntoSchedule, Schedule};

pub struct Cron<Z: chrono::offset::TimeZone> {
    pub iterator: Peekable<OwnedScheduleIterator<Z>>,
    pub schedule: cron::Schedule,
    pub tz: Z,
}

impl<Z: chrono::offset::TimeZone> Cron<Z> {
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
    pub fn from_cron_expr(expr: &str) -> Result<Self, cron::error::Error> {
        let schedule = cron::Schedule::from_str(expr)?;
        Ok(Self::from_cron_schedule(schedule, Utc))
    }
}

impl Cron<Local> {
    pub fn from_cron_expr(expr: &str) -> Result<Self, cron::error::Error> {
        let schedule = cron::Schedule::from_str(expr)?;
        Ok(Self::from_cron_schedule(schedule, Local))
    }
}

impl Schedule for Cron<chrono::Utc> {
    fn peek_next(&mut self) -> Option<Dtu> {
        self.iterator.peek().copied()
    }

    fn next(&mut self) -> Option<Dtu> {
        self.iterator.next()
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
