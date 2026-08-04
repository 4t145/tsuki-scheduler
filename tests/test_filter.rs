//! Real world scenarios for the [`Filtered`] schedule / [`Filter`] / time sets.
//!
//! These tests only rely on the feature-free core, except the ones explicitly gated.
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use chrono::{Datelike, TimeDelta, Timelike, Utc, Weekday};
use tsuki_scheduler::prelude::*;
use tsuki_scheduler::timeset::{Discrete, Functional, Range, TimeSetExt, Universal};

/// Wraps a schedule and counts how many time points it really had to emit,
/// so we can assert that filtering **jumps** instead of walking point by point.
struct Counting<S> {
    inner: S,
    steps: Arc<AtomicUsize>,
}

impl<S: Schedule> Counting<S> {
    fn new(inner: S) -> (Self, Arc<AtomicUsize>) {
        let steps = Arc::new(AtomicUsize::new(0));
        (
            Self {
                inner,
                steps: steps.clone(),
            },
            steps,
        )
    }
}

impl<S: Schedule> Schedule for Counting<S> {
    fn peek_next(&mut self) -> Option<Dtu> {
        self.inner.peek_next()
    }
    fn next(&mut self) -> Option<Dtu> {
        self.steps.fetch_add(1, Ordering::Relaxed);
        self.inner.next()
    }
    fn forward_to(&mut self, dtu: Dtu) {
        self.inner.forward_to(dtu)
    }
}

/// midnight (utc) of the next coming `weekday`, always in the future
fn next_weekday_midnight(weekday: Weekday) -> Dtu {
    let today = Utc::now().date_naive();
    let from_today = (7 + weekday.num_days_from_monday() as i64
        - today.weekday().num_days_from_monday() as i64)
        % 7;
    // add a whole week to make sure it's in the future
    let days = from_today + 7;
    (today + chrono::Days::new(days as u64))
        .and_hms_opt(0, 0, 0)
        .expect("valid time")
        .and_utc()
}

fn is_business_time(dtu: Dtu) -> bool {
    !matches!(dtu.weekday(), Weekday::Sat | Weekday::Sun) && (9..18).contains(&dtu.hour())
}

/// "every workday from 09:00 to 18:00", with a lower bound so that nights and weekends are
/// skipped in a single jump instead of being scanned point by point.
fn business_time() -> Functional {
    Functional::new(is_business_time).with_lower_bound(|from: Dtu| {
        if is_business_time(from) {
            return LowerBound::At(from);
        }
        let mut day = from.date_naive();
        if from.hour() >= 18 {
            // today's window is over
            day = day.succ_opt().expect("date overflow");
        }
        loop {
            if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
                let window_start = day.and_hms_opt(9, 0, 0).expect("valid time").and_utc();
                if window_start >= from {
                    return LowerBound::At(window_start);
                }
            }
            day = day.succ_opt().expect("date overflow");
        }
    })
}

/// A backup job running every 10 minutes must pause during a 24h maintenance window.
///
/// Expressing the exclusion as a union of ranges lets the schedule jump over the whole
/// window at once.
#[test]
fn test_maintenance_window_is_skipped_in_one_jump() {
    let now = Utc::now();
    let maintenance_start = now + TimeDelta::hours(1);
    let maintenance_end = maintenance_start + TimeDelta::hours(24);

    let allowed = Range::before(maintenance_start).union(Range::after(maintenance_end));
    let (inner, steps) = Counting::new(Period::new(TimeDelta::minutes(10), now));
    let mut schedule = inner.filtered_in(allowed);

    let mut runs = vec![];
    for _ in 0..8 {
        runs.push(schedule.next().expect("infinite schedule"));
    }
    // 6 runs before the maintenance window: +0, +10, ... +50 minutes
    assert_eq!(runs[0], now);
    assert_eq!(runs[5], now + TimeDelta::minutes(50));
    // then it resumes right at the end of the window
    assert_eq!(runs[6], maintenance_end);
    assert_eq!(runs[7], maintenance_end + TimeDelta::minutes(10));
    // 8 emitted runs + at most a couple of steps used for the jump,
    // instead of walking through the 144 rejected points of the window
    assert!(
        steps.load(Ordering::Relaxed) <= 10,
        "took {} steps",
        steps.load(Ordering::Relaxed)
    );
}

/// The same exclusion written as a negated filter: same result, but no bound can be derived,
/// so the rejected points are scanned one by one.
#[test]
fn test_negated_filter_is_equivalent_but_scans() {
    let now = Utc::now();
    let maintenance_start = now + TimeDelta::hours(1);
    let maintenance_end = maintenance_start + TimeDelta::hours(24);

    let (inner, steps) = Counting::new(Period::new(TimeDelta::minutes(10), now));
    let mut schedule = inner.filtered(!Filter::in_set(
        Range::between(maintenance_start, maintenance_end).expect("valid range"),
    ));

    let runs: Vec<_> = (0..8)
        .map(|_| schedule.next().expect("infinite schedule"))
        .collect();
    assert_eq!(runs[5], now + TimeDelta::minutes(50));
    assert_eq!(runs[6], maintenance_end);
    // every rejected point had to be visited
    assert!(steps.load(Ordering::Relaxed) > 100);
}

/// A report job runs every 30 minutes, but only during business hours.
#[test]
fn test_business_hours_only() {
    let saturday = next_weekday_midnight(Weekday::Sat);
    let (inner, steps) = Counting::new(Period::new(TimeDelta::minutes(30), saturday));
    let mut schedule = inner.filtered_in(business_time());

    // the weekend is skipped, the first run is monday 09:00
    let monday_9 = saturday + TimeDelta::days(2) + TimeDelta::hours(9);
    assert_eq!(schedule.next(), Some(monday_9));
    // it took a couple of jumps, not 2 days of 30 minutes steps
    assert!(
        steps.load(Ordering::Relaxed) <= 4,
        "took {} steps",
        steps.load(Ordering::Relaxed)
    );

    // 09:00 ..= 17:30, 18 runs a day
    for i in 1..18 {
        assert_eq!(
            schedule.next(),
            Some(monday_9 + TimeDelta::minutes(30 * i)),
            "run {i} of monday"
        );
    }
    // then it jumps to tuesday morning
    assert_eq!(schedule.next(), Some(monday_9 + TimeDelta::days(1)));
}

/// A campaign starts in one year and lasts 5 minutes, the task ticks every second.
///
/// Both the skipping and the termination must be instant.
#[test]
fn test_campaign_window_far_in_the_future() {
    let now = Utc::now();
    let campaign_start = now + TimeDelta::days(365);
    let campaign_end = campaign_start + TimeDelta::minutes(5);
    let mut schedule = Period::new(TimeDelta::seconds(1), now)
        .filtered_in(Range::between(campaign_start, campaign_end).expect("valid range"));

    let start = Instant::now();
    let mut runs = vec![];
    while let Some(next) = schedule.next() {
        runs.push(next);
    }
    let elapsed = start.elapsed();

    assert_eq!(runs.first(), Some(&campaign_start));
    assert_eq!(runs.last(), Some(&(campaign_end - TimeDelta::seconds(1))));
    assert_eq!(runs.len(), 5 * 60);
    // 31 536 000 rejected points before the window, and the schedule is infinite after it:
    // only bounds make this possible
    assert!(elapsed < Duration::from_secs(1), "took {elapsed:?}");
    assert_eq!(schedule.next(), None);
}

/// Only run on an explicit allow list of time points (e.g. billing dates).
#[test]
fn test_allow_list_of_dates() {
    let now = Utc::now();
    let billing_dates: Vec<Dtu> = (1..=3).map(|n| now + TimeDelta::days(30 * n)).collect();
    let (inner, steps) = Counting::new(Period::new(TimeDelta::hours(1), now));
    let mut schedule = inner.filtered_in(Discrete::new(billing_dates.clone()));

    for date in &billing_dates {
        assert_eq!(schedule.next(), Some(*date));
    }
    // the allow list is exhausted, so the whole schedule is over
    assert_eq!(schedule.next(), None);
    assert!(
        steps.load(Ordering::Relaxed) <= 6,
        "took {} steps",
        steps.load(Ordering::Relaxed)
    );
}

/// Combine several conditions: business hours **and** before a deadline.
///
/// The result is checked against a brute force filtering of the same schedule.
#[test]
fn test_composed_filter_matches_brute_force() {
    let saturday = next_weekday_midnight(Weekday::Sat);
    let deadline = saturday + TimeDelta::days(10);

    let filter = Filter::in_set(business_time()) & Filter::in_set(Range::before(deadline));
    let mut schedule = Period::new(TimeDelta::hours(1), saturday).filtered(filter);
    let mut got = vec![];
    while let Some(next) = schedule.next() {
        got.push(next);
    }

    let expected: Vec<Dtu> = (0..)
        .map(|hours| saturday + TimeDelta::hours(hours))
        .take_while(|dtu| *dtu < deadline)
        .filter(|dtu| is_business_time(*dtu))
        .collect();

    assert_eq!(got, expected);
    assert!(!got.is_empty());
    // 6 workdays in [saturday, saturday + 10 days), 9 runs each
    assert_eq!(got.len(), 6 * 9);
}

/// Union of two daily windows: a morning and an evening batch.
#[test]
fn test_any_of_two_windows() {
    let monday = next_weekday_midnight(Weekday::Mon);
    let morning = Range::between(monday + TimeDelta::hours(6), monday + TimeDelta::hours(8))
        .expect("valid range");
    let evening = Range::between(monday + TimeDelta::hours(20), monday + TimeDelta::hours(22))
        .expect("valid range");

    let (inner, steps) = Counting::new(Period::new(TimeDelta::hours(1), monday));
    let mut schedule = inner.filtered(Filter::in_set(morning) | Filter::in_set(evening));

    let runs: Vec<_> = std::iter::from_fn(|| schedule.next()).collect();
    assert_eq!(
        runs,
        vec![
            monday + TimeDelta::hours(6),
            monday + TimeDelta::hours(7),
            monday + TimeDelta::hours(20),
            monday + TimeDelta::hours(21),
        ]
    );
    assert!(
        steps.load(Ordering::Relaxed) <= 8,
        "took {} steps",
        steps.load(Ordering::Relaxed)
    );
}

/// A filtered schedule inside a real [`Scheduler`].
#[test]
fn test_scheduler_only_runs_inside_window() {
    let now = Utc::now();
    let window_start = now + TimeDelta::seconds(30);
    let window_end = now + TimeDelta::seconds(60);
    let count = Arc::new(AtomicUsize::new(0));

    let schedule = Period::new(TimeDelta::seconds(10), now)
        .filtered_in(Range::between(window_start, window_end).expect("valid range"));
    let mut scheduler = Scheduler::new(Local::new());
    let task = {
        let count = count.clone();
        Task::local(schedule, move || {
            count.fetch_add(1, Ordering::SeqCst);
        })
    };
    scheduler.add_task(TaskUid::new(0), task);

    // nothing before the window
    scheduler.execute(now + TimeDelta::seconds(20));
    assert_eq!(count.load(Ordering::SeqCst), 0);
    // +30s, +40s, +50s inside the window
    scheduler.execute(now + TimeDelta::seconds(70));
    assert_eq!(count.load(Ordering::SeqCst), 3);
    // the schedule is over, the task has been dropped
    scheduler.execute(now + TimeDelta::days(1));
    assert_eq!(count.load(Ordering::SeqCst), 3);
    assert!(scheduler.delete_task(TaskUid::new(0)).is_none());
}

/// Filters compose with the other combinators.
#[test]
fn test_filter_with_other_combinators() {
    let saturday = next_weekday_midnight(Weekday::Sat);
    let monday_9 = saturday + TimeDelta::days(2) + TimeDelta::hours(9);
    // every 15 minutes during business hours, but at most one run per hour,
    // and never after monday 13:00
    let mut schedule = Period::new(TimeDelta::minutes(15), saturday)
        .filtered_in(business_time())
        .throttling(TimeDelta::hours(1))
        .before(monday_9 + TimeDelta::hours(4));

    let runs: Vec<_> = std::iter::from_fn(|| schedule.next()).collect();
    // the throttle keeps at most one run per hour: 09:00, 10:00, 11:00, 12:00
    let expected: Vec<Dtu> = (0..4).map(|i| monday_9 + TimeDelta::hours(i)).collect();
    assert_eq!(runs, expected);
    for run in &runs {
        assert!(is_business_time(*run), "{run} is not a business time");
    }

    // the same chain over a whole day: the throttle never leaks a run outside of the business
    // hours nor past the deadline (tuesday 00:00)
    let mut schedule = Period::new(TimeDelta::minutes(15), saturday)
        .filtered_in(business_time())
        .throttling(TimeDelta::hours(1))
        .before(saturday + TimeDelta::days(3));
    let runs: Vec<_> = std::iter::from_fn(|| schedule.next()).collect();
    // monday 09:00 ..= 17:00
    let expected: Vec<Dtu> = (0..9).map(|i| monday_9 + TimeDelta::hours(i)).collect();
    assert_eq!(runs, expected);
}

/// Exclude a black list of time points from an otherwise unrestricted schedule.
#[cfg(feature = "cron")]
#[test]
fn test_cron_with_holiday_black_list() {
    let today = Utc::now().date_naive();
    let noon = |days: u64| {
        (today + chrono::Days::new(days))
            .and_hms_opt(12, 0, 0)
            .expect("valid time")
            .and_utc()
    };
    // the next 3 noons after tomorrow are holidays
    let holidays: Vec<Dtu> = (2..=4).map(noon).collect();
    let mut schedule = Cron::utc_from_cron_expr("0 0 12 * * *")
        .expect("invalid cron")
        .filtered_in(Universal.difference(Discrete::new(holidays.clone())));

    let runs: Vec<Dtu> = (0..4)
        .map(|_| schedule.next().expect("cron never ends"))
        .collect();
    for run in &runs {
        assert_eq!(run.hour(), 12);
        assert!(!holidays.contains(run), "{run} is a holiday");
    }
    // holidays are skipped, so the 4 runs span more than 4 days
    assert!(*runs.last().expect("not empty") >= noon(5));
}
