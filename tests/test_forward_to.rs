use chrono::{TimeDelta, Utc};

#[test]
fn test_forward_to() {
    use tsuki_scheduler::prelude::*;
    let now = Utc::now();
    let mut period = Period::new(TimeDelta::days(10), now);
    period.forward_to(now + TimeDelta::days(7));
    assert_eq!(
        chrono::DurationRound::duration_round(period.get_next(), TimeDelta::milliseconds(1))
            .unwrap(),
        chrono::DurationRound::duration_round(
            Utc::now() + TimeDelta::days(10),
            TimeDelta::milliseconds(1)
        )
        .unwrap()
    );

    let mut period = Period::new(TimeDelta::new(30, 30_123_456).unwrap(), now);
    period.forward_to(now + TimeDelta::days(1));
    let time_forwarded = (period.get_next() - now).num_hours();
    assert!((23..=25).contains(&time_forwarded));
}
