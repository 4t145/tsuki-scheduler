use chrono::{DateTime, TimeDelta};
use tsuki_scheduler::prelude::*;

#[test]
pub fn test_before_and_after() {
    // test after, before
    let day_0 = DateTime::parse_from_rfc3339("2025-01-01T00:00:00-08:00")
        .expect("invalid")
        .to_utc();
    let day_0_noon = DateTime::parse_from_rfc3339("2025-01-01T12:00:00-08:00")
        .expect("invalid")
        .to_utc();
    let day_1 = DateTime::parse_from_rfc3339("2025-01-02T00:00:00-08:00")
        .expect("invalid")
        .to_utc();
    let day_2 = DateTime::parse_from_rfc3339("2025-01-02T00:00:00-08:00")
        .expect("invalid")
        .to_utc();
    let day_3_noon = DateTime::parse_from_rfc3339("2025-01-03T12:00:00-08:00")
        .expect("invalid")
        .to_utc();
    let day_4 = DateTime::parse_from_rfc3339("2025-01-05T00:00:00-08:00")
        .expect("invalid")
        .to_utc();

    let schedule = Iter::new([day_0, day_1, day_2, day_4]);
    let mut schedule = schedule.after(day_0_noon).before(day_3_noon);
    assert_eq!(schedule.next(), Some(day_1));
    assert_eq!(schedule.next(), Some(day_2));
    assert_eq!(schedule.next(), None);
}

#[test]
pub fn test_then() {
    // test then
    let day_0 = DateTime::parse_from_rfc3339("2025-01-01T00:00:00-08:00")
        .expect("invalid")
        .to_utc();
    let day_1 = DateTime::parse_from_rfc3339("2025-01-02T00:00:00-08:00")
        .expect("invalid")
        .to_utc();
    let day_2 = DateTime::parse_from_rfc3339("2025-01-03T00:00:00-08:00")
        .expect("invalid")
        .to_utc();
    let day_3 = DateTime::parse_from_rfc3339("2025-01-04T00:00:00-08:00")
        .expect("invalid")
        .to_utc();
    let day_4 = DateTime::parse_from_rfc3339("2025-01-05T00:00:00-08:00")
        .expect("invalid")
        .to_utc();

    let schedule = Iter::new([day_0, day_1, day_2]);
    let mut schedule = schedule.then(Iter::new([day_0, day_1, day_2, day_3, day_4]));
    assert_eq!(schedule.next(), Some(day_0));
    assert_eq!(schedule.next(), Some(day_1));
    assert_eq!(schedule.next(), Some(day_2));
    assert_eq!(schedule.next(), Some(day_3));
    assert_eq!(schedule.next(), Some(day_4));
    assert_eq!(schedule.next(), None);
}

#[test]
pub fn test_or() {
    // test or
    let day_0 = DateTime::parse_from_rfc3339("2025-01-01T00:00:00-08:00")
        .expect("invalid")
        .to_utc();
    let day_1 = DateTime::parse_from_rfc3339("2025-01-02T00:00:00-08:00")
        .expect("invalid")
        .to_utc();
    let day_2 = DateTime::parse_from_rfc3339("2025-01-03T00:00:00-08:00")
        .expect("invalid")
        .to_utc();
    let day_3 = DateTime::parse_from_rfc3339("2025-01-04T00:00:00-08:00")
        .expect("invalid")
        .to_utc();
    let day_4 = DateTime::parse_from_rfc3339("2025-01-05T00:00:00-08:00")
        .expect("invalid")
        .to_utc();

    let schedule = Iter::new([day_0, day_2, day_4]);
    let mut schedule = schedule.or(Iter::new([day_1, day_3]));
    assert_eq!(schedule.next(), Some(day_0));
    assert_eq!(schedule.next(), Some(day_1));
    assert_eq!(schedule.next(), Some(day_2));
    assert_eq!(schedule.next(), Some(day_3));
    assert_eq!(schedule.next(), Some(day_4));
    assert_eq!(schedule.next(), None);
}

#[test]
pub fn test_period() {
    let day_0 = now();
    let delta = TimeDelta::days(1);
    let schedule = Period::new(delta, day_0);
    let mut schedule = schedule.into_schedule();
    assert_eq!(schedule.next(), Some(day_0));
    assert_eq!(schedule.next(), Some(day_0 + delta));
    assert_eq!(schedule.next(), Some(day_0 + delta * 2));
    assert_eq!(schedule.next(), Some(day_0 + delta * 3));
    assert_eq!(schedule.next(), Some(day_0 + delta * 4));
}

#[test]
pub fn test_once() {
    let day_0 = now();
    let schedule = Once::new(day_0);
    let mut schedule = schedule.into_schedule();
    assert_eq!(schedule.next(), Some(day_0));
    assert_eq!(schedule.next(), None);
}

#[test]
pub fn test_throttling() {
    let day_0 = now();
    let delta = TimeDelta::days(1);
    let schedule = Period::new(delta, day_0);
    let schedule = Throttling::new(schedule, TimeDelta::days(2));
    let mut schedule = schedule.into_schedule();
    assert_eq!(schedule.next(), Some(day_0));
    assert_eq!(schedule.next(), Some(day_0 + delta * 2));
    assert_eq!(schedule.next(), Some(day_0 + delta * 4));
    assert_eq!(schedule.next(), Some(day_0 + delta * 6));
    assert_eq!(schedule.next(), Some(day_0 + delta * 8));
}

// I want to create a schedule:
// 1. firstly it will run at 10 seconds later,
// 2. and then, it will run at every hour's 10th minute,
// 3. meanwhile, it will run every 80 minutes,
// 4. though, it won't run within 30 minutes after the last run.
// 5. finally, it will stop running after 100 days later.
#[cfg(feature = "cron")]
#[test]
pub fn test_complex_example() {
    let start_time = now() + TimeDelta::seconds(10);
    let schedule = Once::new(start_time)
        .then(
            Cron::utc_from_cron_expr("00 10 * * * *")
                .expect("invalid cron")
                .or(Period::new(
                    TimeDelta::minutes(80),
                    start_time + TimeDelta::minutes(80),
                ))
                .throttling(TimeDelta::minutes(30)),
        )
        .before(start_time + TimeDelta::days(100));
    let mut _schedule = schedule.into_schedule();
    // I don't want to run this test forever, so I will just check the first 10 runs.
    for _ in 0..10 {
        println!("{:?}", _schedule.next());
    }
}

/// `peek_next` must always agree with `next`: the throttle may only drop time points of the
/// inner schedule, never invent one.
///
/// Regression test: it used to return the synthesized `last_call + interval`, which could leak
/// a run past the limit of an outer combinator such as `before`.
#[test]
pub fn test_throttling_peek_matches_next() {
    let day_0 = now();
    let hour = TimeDelta::hours(1);
    // runs every 15 minutes, at most one run per hour
    let mut schedule = Period::new(TimeDelta::minutes(15), day_0).throttling(hour);
    for i in 0..5 {
        let peeked = schedule.peek_next();
        assert_eq!(peeked, Some(day_0 + hour * i));
        assert_eq!(schedule.next(), peeked, "peek and next disagree at run {i}");
    }

    // the last time point of the inner schedule before the throttle deadline used to be
    // reported as `last_call + interval`, letting `before` accept a run it should reject
    let mut schedule = Period::new(TimeDelta::minutes(15), day_0)
        .throttling(hour)
        .before(day_0 + TimeDelta::minutes(30));
    assert_eq!(schedule.next(), Some(day_0));
    assert_eq!(schedule.peek_next(), None);
    assert_eq!(schedule.next(), None);
}

/// A time point landing exactly on `last_call + interval` is still allowed.
#[test]
pub fn test_throttling_boundary_is_inclusive() {
    let day_0 = now();
    let delta = TimeDelta::days(1);
    let mut schedule = Period::new(delta, day_0).throttling(delta);
    for i in 0..5 {
        assert_eq!(schedule.next(), Some(day_0 + delta * i));
    }
}
