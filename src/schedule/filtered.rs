use crate::{
    Dtu,
    schedule::Schedule,
    timeset::{DynTimeSet, LowerBound, TimeSet, pred_dtu},
};

/// A wrapper around a schedule that only keeps the time points accepted by a [`Filter`].
///
/// # Skipping strategy
/// When a time point is rejected, the filter is asked for a [`LowerBound`], and:
/// - [`LowerBound::At(bound)`](LowerBound::At): the inner schedule is forwarded right before
///   `bound` in **one** [`forward_to`](Schedule::forward_to) call, so skipping a year of a
///   one-second-period schedule costs `O(1)` instead of 31,536,000 steps.
/// - [`LowerBound::Never`]: the schedule is exhausted, [`next`](Schedule::next) returns `None`
///   instead of looping forever.
/// - [`LowerBound::Unknown`]: fall back to consuming time points one by one.
///
/// So the cost of filtering depends on how good the [`TimeSet`] bounds are. Prefer
/// [`Range`](crate::timeset::Range) / [`Discrete`](crate::timeset::Discrete) /
/// [`Union`](crate::timeset::Union) / [`Intersection`](crate::timeset::Intersection) over a bare
/// [`Functional`](crate::timeset::Functional) predicate, or attach a bound to the predicate with
/// [`Functional::with_lower_bound`](crate::timeset::Functional::with_lower_bound).
///
/// # Warning
/// If the inner schedule is infinite and the filter rejects everything while reporting
/// [`LowerBound::Unknown`], `peek_next` / `next` will loop forever. Bound such schedules with
/// [`Before`](super::Before), or make the time set report its bounds.
pub struct Filtered<S> {
    pub inner: S,
    pub filter: Filter,
}

impl<S> Filtered<S> {
    pub fn new(inner: S, filter: Filter) -> Self {
        Self { inner, filter }
    }
    /// Only keep the time points inside `set`.
    pub fn in_set<T: TimeSet>(inner: S, set: T) -> Self {
        Self::new(inner, Filter::in_set(set))
    }
    pub fn inner(&self) -> &S {
        &self.inner
    }
    pub fn filter(&self) -> &Filter {
        &self.filter
    }
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Schedule> Filtered<S> {
    /// Skip all the rejected time points, so that the inner schedule is peeking at an accepted
    /// time point (or is exhausted / will never be accepted again).
    fn skip_rejected(&mut self) -> Option<Dtu> {
        loop {
            let next = self.inner.peek_next()?;
            if self.filter.matches(next) {
                return Some(next);
            }
            match self.filter.lower_bound(next) {
                // nothing will ever be accepted again
                LowerBound::Never => return None,
                // jump in one step: forward right before the bound, so `bound` itself is kept
                LowerBound::At(bound) if bound > next => {
                    let target = pred_dtu(bound).unwrap_or(bound);
                    self.inner.forward_to(target);
                    // the inner schedule may not honor `forward_to`, always ensure progress
                    if self.inner.peek_next().is_some_and(|peek| peek <= next) {
                        self.inner.next()?;
                    }
                }
                // no usable bound, walk one time point forward
                _ => {
                    self.inner.next()?;
                }
            }
        }
    }
}

impl<S: Schedule> Schedule for Filtered<S> {
    fn peek_next(&mut self) -> Option<Dtu> {
        self.skip_rejected()
    }

    fn next(&mut self) -> Option<Dtu> {
        self.skip_rejected()?;
        self.inner.next()
    }

    fn forward_to(&mut self, dtu: Dtu) {
        self.inner.forward_to(dtu)
    }
}

/// A predicate over time points, see [`FilterKind`] for the supported combinators.
///
/// A [`Filter`] is itself a [`TimeSet`], so filters can be nested into time sets and vice versa.
pub struct Filter {
    pub kind: FilterKind,
}

impl Filter {
    pub fn new(kind: FilterKind) -> Self {
        Self { kind }
    }
    /// Accept the time points contained by `set`.
    pub fn in_set<T: TimeSet>(set: T) -> Self {
        Self::new(FilterKind::In(Box::new(set)))
    }
    /// Accept the time points rejected by `self`.
    ///
    /// Also available as [`std::ops::Not`] (`!filter`).
    ///
    /// # Note
    /// The complement of a time set cannot report a useful [`LowerBound`], a negated filter
    /// always falls back to point-by-point scanning. Prefer expressing the negation inside the
    /// time set itself (e.g. [`Difference`](crate::timeset::Difference)) when performance matters.
    pub fn negate(self) -> Self {
        Self::new(FilterKind::Not(Box::new(self)))
    }
    /// Accept the time points accepted by all the `filters`.
    ///
    /// An empty iterator accepts everything.
    pub fn all<I: IntoIterator<Item = Filter>>(filters: I) -> Self {
        Self::new(FilterKind::All(filters.into_iter().collect()))
    }
    /// Accept the time points accepted by any of the `filters`.
    ///
    /// An empty iterator accepts nothing.
    pub fn any<I: IntoIterator<Item = Filter>>(filters: I) -> Self {
        Self::new(FilterKind::Any(filters.into_iter().collect()))
    }
    /// Accept the time points accepted by both `self` and `other`.
    ///
    /// Also available as [`std::ops::BitAnd`] (`a & b`).
    pub fn and(self, other: Filter) -> Self {
        Self::all([self, other])
    }
    /// Accept the time points accepted by `self` or `other`.
    ///
    /// Also available as [`std::ops::BitOr`] (`a | b`).
    pub fn or(self, other: Filter) -> Self {
        Self::any([self, other])
    }
    /// Whether the time point `dtu` is accepted by this filter.
    pub fn matches(&self, dtu: Dtu) -> bool {
        match &self.kind {
            FilterKind::Not(filter) => !filter.matches(dtu),
            FilterKind::All(filters) => filters.iter().all(|filter| filter.matches(dtu)),
            FilterKind::Any(filters) => filters.iter().any(|filter| filter.matches(dtu)),
            FilterKind::In(set) => set.contains(dtu),
        }
    }
    /// The lower bound of the next accepted time point at or after `from`, see [`LowerBound`].
    pub fn bound(&self, from: Dtu) -> LowerBound {
        match &self.kind {
            // the complement of a set gives no bound
            FilterKind::Not(_) => LowerBound::Unknown,
            // all of them must accept, so the latest bound wins
            FilterKind::All(filters) => {
                LowerBound::meet_all(filters.iter().map(|filter| filter.bound(from)))
            }
            // any of them may accept, so the earliest bound wins
            FilterKind::Any(filters) => {
                LowerBound::join_all(filters.iter().map(|filter| filter.bound(from)))
            }
            FilterKind::In(set) => set.lower_bound(from),
        }
    }
}

impl TimeSet for Filter {
    fn contains(&self, dtu: Dtu) -> bool {
        self.matches(dtu)
    }
    fn lower_bound(&self, from: Dtu) -> LowerBound {
        self.bound(from)
    }
}

impl std::ops::Not for Filter {
    type Output = Filter;
    fn not(self) -> Self::Output {
        self.negate()
    }
}

impl std::ops::BitAnd for Filter {
    type Output = Filter;
    fn bitand(self, rhs: Self) -> Self::Output {
        self.and(rhs)
    }
}

impl std::ops::BitOr for Filter {
    type Output = Filter;
    fn bitor(self, rhs: Self) -> Self::Output {
        self.or(rhs)
    }
}

pub enum FilterKind {
    Not(Box<Filter>),
    All(Vec<Filter>),
    Any(Vec<Filter>),
    In(DynTimeSet),
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use chrono::{TimeDelta, Utc};

    use super::*;
    use crate::{
        schedule::{Period, ScheduleExt},
        timeset::{Discrete, Functional, Range, TimeSetExt, Union},
    };

    /// counts how many time points the inner schedule really emitted
    struct Counting<S> {
        inner: S,
        steps: Arc<AtomicUsize>,
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

    fn every_second() -> Period {
        Period::new(TimeDelta::seconds(1), Utc::now())
    }

    #[test]
    fn test_skip_one_year_in_constant_steps() {
        let now = Utc::now();
        let start = now + TimeDelta::days(365);
        let steps = Arc::new(AtomicUsize::new(0));
        let inner = Counting {
            inner: every_second(),
            steps: steps.clone(),
        };
        let mut filtered = Filtered::in_set(inner, Range::after(start));
        let next = filtered.next().expect("should have next");
        assert!(next >= start);
        assert!(next < start + TimeDelta::seconds(1));
        // one jump + one emit, definitely not 31_536_000 steps
        assert!(steps.load(Ordering::Relaxed) <= 2);
    }

    #[test]
    fn test_never_terminates_instead_of_hanging() {
        let now = Utc::now();
        // an infinite dense schedule, but the time set is over
        let mut filtered = Filtered::in_set(every_second(), Range::before(now));
        assert_eq!(filtered.next(), None);
        assert_eq!(filtered.peek_next(), None);
    }

    #[test]
    fn test_jump_between_windows() {
        let now = Utc::now();
        let day = |n: i64| now + TimeDelta::days(n);
        let steps = Arc::new(AtomicUsize::new(0));
        let windows = Union::new(vec![
            Range::between(day(100), day(100) + TimeDelta::seconds(2))
                .expect("valid")
                .dyn_box(),
            Range::between(day(200), day(200) + TimeDelta::seconds(2))
                .expect("valid")
                .dyn_box(),
        ]);
        let inner = Counting {
            inner: every_second(),
            steps: steps.clone(),
        };
        let mut filtered = Filtered::in_set(inner, windows);
        let mut hits = vec![];
        while let Some(next) = filtered.next() {
            hits.push(next);
        }
        // 2 time points per window
        assert_eq!(hits.len(), 4);
        assert!(hits[0] >= day(100) && hits[1] < day(100) + TimeDelta::seconds(2));
        assert!(hits[2] >= day(200) && hits[3] < day(200) + TimeDelta::seconds(2));
        // 4 emits + a few jumps, not millions of steps
        assert!(steps.load(Ordering::Relaxed) <= 8);
    }

    #[test]
    fn test_discrete_filter_keeps_matched_only() {
        let now = Utc::now();
        let period = Period::new(TimeDelta::seconds(1), now);
        let wanted = [
            now + TimeDelta::seconds(3),
            now + TimeDelta::seconds(300_000),
        ];
        let mut filtered = period.filtered_in(Discrete::new(wanted));
        assert_eq!(filtered.next(), Some(wanted[0]));
        assert_eq!(filtered.next(), Some(wanted[1]));
        assert_eq!(filtered.next(), None);
    }

    #[test]
    fn test_unknown_bound_falls_back_to_scanning() {
        let now = Utc::now();
        let start = now - TimeDelta::nanoseconds(now.timestamp_subsec_nanos() as i64);
        let period = Period::new(TimeDelta::seconds(1), start);
        // a bare predicate cannot provide any bound, so it is scanned point by point
        let even = Functional::new(|dtu: Dtu| dtu.timestamp() % 2 == 0);
        let mut filtered = Filtered::in_set(period, even);
        let first = filtered.next().expect("should have next");
        let second = filtered.next().expect("should have next");
        assert_eq!(first.timestamp() % 2, 0);
        assert_eq!(second - first, TimeDelta::seconds(2));
    }

    #[test]
    fn test_filter_combinators() {
        let now = Utc::now();
        let always = || Filter::in_set(Functional::new(|_| true));
        let never = || Filter::in_set(Functional::new(|_| false));
        assert!(always().matches(now));
        assert!(!(!always()).matches(now));
        assert!((always() & always()).matches(now));
        assert!(!(always() & never()).matches(now));
        assert!((never() | always()).matches(now));
        // identity elements
        assert!(Filter::all([]).matches(now));
        assert!(!Filter::any([]).matches(now));

        // bound combination
        let after = |n: i64| Filter::in_set(Range::after(now + TimeDelta::days(n)));
        assert_eq!(
            (after(1) & after(10)).bound(now),
            LowerBound::At(now + TimeDelta::days(10))
        );
        assert_eq!(
            (after(1) | after(10)).bound(now),
            LowerBound::At(now + TimeDelta::days(1))
        );
        assert_eq!((!after(1)).bound(now), LowerBound::Unknown);
        assert_eq!(
            (after(1) & never()).bound(now),
            LowerBound::At(now + TimeDelta::days(1))
        );
        assert_eq!((after(1) | never()).bound(now), LowerBound::Unknown);
    }
}
