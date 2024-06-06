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
    let s_r = s_x % (s_p + 1) - (n_p * q_0) / NANOS_PER_SEC;
    let n_r = n_x % n_p + ((NANOS_PER_SEC % n_p) * ((s_r) % n_p) % n_p);
    let x = TimeDelta::new(s_r, n_r as u32).expect("invalid time delta");
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

    fn forward(&mut self, dtu: Dtu) {
        if self.next < dtu {
            let diff = dtu - self.next;
            if diff < self.period {
                self.next += self.period;
                return;
            }

            todo!()
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