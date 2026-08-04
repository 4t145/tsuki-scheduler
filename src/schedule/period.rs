use chrono::{TimeDelta, Utc};

use super::{IntoSchedule, Schedule};
use crate::Dtu;

/// A schedule that runs at a fixed interval.
#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq)]
pub struct Period {
    period: TimeDelta,
    next: Dtu,
}

impl Period {
    pub fn new(period: TimeDelta, from: Dtu) -> Self {
        assert!(period > TimeDelta::zero(), "Period must be positive");
        assert!(
            from > Utc::now() - period,
            "start time must be in the future"
        );
        Self { period, next: from }
    }
    pub fn period(&self) -> TimeDelta {
        self.period
    }
    pub fn get_next(&self) -> Dtu {
        self.next
    }
}

const NANOS_PER_SEC: i64 = 1_000_000_000;

/// total nanoseconds of a [`TimeDelta`], in `i128` to never overflow
fn total_nanos(delta: TimeDelta) -> i128 {
    delta.num_seconds() as i128 * NANOS_PER_SEC as i128 + delta.subsec_nanos() as i128
}

/// `x mod p`, exact at nanosecond resolution
fn time_mod(x: TimeDelta, p: TimeDelta) -> TimeDelta {
    let p_nanos = total_nanos(p);
    if p_nanos <= 0 {
        panic!("Period must be positive")
    }
    if x < p {
        return x;
    }
    let rest = total_nanos(x) % p_nanos;
    TimeDelta::new(
        (rest / NANOS_PER_SEC as i128) as i64,
        (rest % NANOS_PER_SEC as i128) as u32,
    )
    .expect("remainder is always a valid time delta")
}
impl Schedule for Period {
    fn peek_next(&mut self) -> Option<Dtu> {
        Some(self.next)
    }

    fn next(&mut self) -> Option<Dtu> {
        let next = self.next;
        self.next += self.period;
        Some(next)
    }

    fn forward_to(&mut self, dtu: Dtu) {
        if self.next < dtu {
            let diff = dtu - self.next;
            if diff < self.period {
                self.next += self.period;
                return;
            }
            let rest = time_mod(diff, self.period);
            self.next = dtu + self.period - rest;
        }
    }
}

impl IntoSchedule for TimeDelta {
    type Output = Period;
    fn into_schedule(self) -> Self::Output {
        Period::new(self, Utc::now())
    }
}
