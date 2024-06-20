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
