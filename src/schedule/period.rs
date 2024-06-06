use chrono::{DateTime, DurationRound, TimeDelta, Utc};

use crate::{Dtu, IntoSchedule, Schedule};

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
    pub fn next(&self) -> Dtu {
        self.next
    }
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

    fn forward(&mut self, dtu: Dtu) {
        if self.next < dtu {
            let diff = dtu - self.next + self.period;
            let s_d = diff.num_seconds();
            let s_p = self.period.num_seconds();

            let n_d = diff.subsec_nanos();
            let n_p = self.period.subsec_nanos();
            if n_p == 0 {
                if s_p == 0 {
    
                }
                self.next = dtu + TimeDelta::seconds(s_d - (s_d % s_p));
            } else {
                let r_s = s_d % s_p;
                const NANOS_PER_SEC: i32 = 1_000_000_000;
                let r_n = (n_d % n_p) + ((NANOS_PER_SEC % n_p) * ((r_s as i32) % n_p) % n_p);
                self.next = dtu + TimeDelta::seconds(r_s) + TimeDelta::nanoseconds(r_n as i64);
            }
        }
    }
}

impl IntoSchedule for TimeDelta {
    type Output = Period;
    fn into_schedule(self) -> Self::Output {
        Period::new(self, Utc::now())
    }
}

impl IntoSchedule for Period {
    type Output = Period;
    fn into_schedule(self) -> Self::Output {
        self
    }
}

#[test]
fn test_forward() {
    let now = Utc::now();
    let mut period = Period::new(TimeDelta::days(10), now);
    period.forward(now + TimeDelta::days(7));
    assert_eq!(period.next(), Utc::now() + TimeDelta::days(10));
}
