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

fn time_mod(x: TimeDelta, p: TimeDelta) -> TimeDelta {
    if p == TimeDelta::zero() {
        panic!("Period must be positive")
    }
    if x < p {
        return x;
    }
    let s_x = x.num_seconds();
    let n_x = x.subsec_nanos() as i64;
    let s_p = p.num_seconds();
    let n_p = p.subsec_nanos() as i64;
    const NANOS_PER_SEC: i64 = 1_000_000_000;
    if s_p == 0 {
        let nanos_in_total = (n_x % n_p) + ((NANOS_PER_SEC % n_p) * ((s_x) % n_p) % n_p);
        let secs = nanos_in_total / NANOS_PER_SEC;
        let nanos = (nanos_in_total % NANOS_PER_SEC) as u32;
        return TimeDelta::new(secs, nanos).expect("invalid time delta");
    }
    let q_0 = s_x / (s_p + 1);
    let s_r = (s_x % (s_p + 1)) + ((n_p * q_0) / NANOS_PER_SEC);
    let n_r = n_x % n_p + ((NANOS_PER_SEC % n_p) * (s_r % n_p) % n_p);
    let x = TimeDelta::new(s_r, n_r as u32)
        .ok_or((s_r, n_r))
        .expect("invalid time delta");
    time_mod(x, p)
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

