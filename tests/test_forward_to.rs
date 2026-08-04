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

/// `forward_to` must keep the phase of the period, whatever the alignment of the target is.
///
/// Regression test: the remainder computation used to be wrong for whole-second periods,
/// which shifted every following run by an arbitrary amount.
#[test]
fn test_forward_to_keeps_phase() {
    use tsuki_scheduler::prelude::*;
    let start = Utc::now();

    for period in [
        TimeDelta::minutes(10),
        TimeDelta::seconds(1),
        TimeDelta::hours(3),
        TimeDelta::new(30, 30_123_456).expect("valid delta"),
    ] {
        for target in [
            start + TimeDelta::days(1),
            start + TimeDelta::days(365),
            start + period * 3,
            start + period * 3 - TimeDelta::nanoseconds(1),
            start + TimeDelta::hours(25) + TimeDelta::nanoseconds(7),
        ] {
            let mut schedule = Period::new(period, start);
            schedule.forward_to(target);
            let next = schedule.get_next();
            // strictly after the target, but not further than one period
            assert!(next > target, "{next} <= {target} (period {period})");
            assert!(next <= target + period, "{next} > {target} + {period}");
            // and still in phase with the original start
            let offset = next - start;
            assert_eq!(
                offset.num_nanoseconds().expect("no overflow")
                    % period.num_nanoseconds().expect("no overflow"),
                0,
                "out of phase: {next} (start {start}, period {period}, target {target})"
            );
        }
    }
}
