use std::{collections::BTreeSet, sync::Arc};

use crate::Dtu;

pub type DynTimeSet = Box<dyn TimeSet>;

/// The **lower bound** of the next time point that may be contained by a [`TimeSet`].
///
/// This is the key to skipping efficiently: instead of testing every single time point of a
/// dense schedule, a [`TimeSet`] can tell "nothing before `bound` is in me", so the consumer
/// can jump directly to `bound`.
///
/// # Contract
/// For `set.lower_bound(from)`:
/// - [`LowerBound::At(bound)`](LowerBound::At): `bound >= from` and **no** time point in
///   `[from, bound)` is contained by the set. `bound` itself is only a *candidate*, it does not
///   have to be contained.
/// - [`LowerBound::Never`]: no time point in `[from, +∞)` is contained by the set.
/// - [`LowerBound::Unknown`]: no useful bound can be computed, the consumer should fall back
///   to testing time points one by one.
///
/// Returning [`LowerBound::Unknown`] is always sound, `At(from)` is always sound too, they are
/// just useless. The tighter the bound, the faster the skipping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowerBound {
    At(Dtu),
    Never,
    Unknown,
}

impl LowerBound {
    /// The bound for an intersection-like combination (all of the sets must contain the point).
    ///
    /// Takes the **latest** known bound, since every member's bound is a necessary condition.
    /// [`Never`](LowerBound::Never) is absorbing, [`Unknown`](LowerBound::Unknown) is neutral.
    pub fn meet(self, other: Self) -> Self {
        match (self, other) {
            (Self::Never, _) | (_, Self::Never) => Self::Never,
            (Self::At(a), Self::At(b)) => Self::At(a.max(b)),
            (Self::At(a), Self::Unknown) | (Self::Unknown, Self::At(a)) => Self::At(a),
            (Self::Unknown, Self::Unknown) => Self::Unknown,
        }
    }
    /// The bound for a union-like combination (any of the sets may contain the point).
    ///
    /// Takes the **earliest** known bound, since any member may contain the point.
    /// [`Unknown`](LowerBound::Unknown) is absorbing, [`Never`](LowerBound::Never) is neutral.
    pub fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            (Self::At(a), Self::At(b)) => Self::At(a.min(b)),
            (Self::At(a), Self::Never) | (Self::Never, Self::At(a)) => Self::At(a),
            (Self::Never, Self::Never) => Self::Never,
        }
    }
    /// [`meet`](LowerBound::meet) over an iterator, empty iterator yields
    /// [`Unknown`](LowerBound::Unknown).
    pub fn meet_all<I: IntoIterator<Item = Self>>(bounds: I) -> Self {
        bounds.into_iter().fold(Self::Unknown, Self::meet)
    }
    /// [`join`](LowerBound::join) over an iterator, empty iterator yields
    /// [`Never`](LowerBound::Never).
    pub fn join_all<I: IntoIterator<Item = Self>>(bounds: I) -> Self {
        bounds.into_iter().fold(Self::Never, Self::join)
    }
    /// The bound as a time point, if any.
    pub fn at(self) -> Option<Dtu> {
        match self {
            Self::At(dtu) => Some(dtu),
            _ => None,
        }
    }
}

/// The smallest representable time point, used as the origin when caching bounds.
pub fn min_dtu() -> Dtu {
    chrono::DateTime::<chrono::Utc>::MIN_UTC
}

/// The next representable time point after `dtu` (`chrono` has nanosecond resolution).
pub fn succ_dtu(dtu: Dtu) -> Option<Dtu> {
    dtu.checked_add_signed(chrono::TimeDelta::nanoseconds(1))
}

/// The previous representable time point before `dtu`.
pub fn pred_dtu(dtu: Dtu) -> Option<Dtu> {
    dtu.checked_sub_signed(chrono::TimeDelta::nanoseconds(1))
}

/// A set of time points.
///
/// Beside the membership test [`contains`](TimeSet::contains), an implementation should provide
/// [`lower_bound`](TimeSet::lower_bound) whenever it can be computed cheaply, so that consumers
/// (e.g. [`Filtered`](crate::schedule::Filtered)) can skip long rejected intervals in one step
/// instead of walking through every time point.
pub trait TimeSet: Send + 'static {
    fn contains(&self, dtu: Dtu) -> bool;
    /// The lower bound of the next contained time point at or after `from`, see [`LowerBound`].
    ///
    /// The default implementation gives up with [`LowerBound::Unknown`].
    fn lower_bound(&self, from: Dtu) -> LowerBound {
        let _ = from;
        LowerBound::Unknown
    }
}

impl TimeSet for DynTimeSet {
    fn contains(&self, dtu: Dtu) -> bool {
        self.as_ref().contains(dtu)
    }
    fn lower_bound(&self, from: Dtu) -> LowerBound {
        self.as_ref().lower_bound(from)
    }
}

/// shortcuts for combining time sets
pub trait TimeSetExt: TimeSet + Sized {
    fn dyn_box(self) -> DynTimeSet {
        Box::new(self)
    }
    fn union<T: TimeSet>(self, other: T) -> Union {
        Union::new(vec![self.dyn_box(), other.dyn_box()])
    }
    fn intersection<T: TimeSet>(self, other: T) -> Intersection {
        Intersection::new(vec![self.dyn_box(), other.dyn_box()])
    }
    fn difference<T: TimeSet>(self, other: T) -> Difference<Self, T> {
        Difference { a: self, b: other }
    }
}

impl<T: TimeSet + Sized> TimeSetExt for T {}

/// A time set defined by an arbitrary predicate.
///
/// It cannot compute any [`LowerBound`], use [`Functional::with_lower_bound`] to attach one if
/// the predicate is periodic / monotonic in some way, otherwise consumers have to scan time
/// points one by one.
#[derive(Clone)]
pub struct Functional {
    pub contains: Arc<dyn Fn(Dtu) -> bool + Send + Sync>,
    pub lower_bound: Option<Arc<dyn Fn(Dtu) -> LowerBound + Send + Sync>>,
}

impl TimeSet for Functional {
    fn contains(&self, dtu: Dtu) -> bool {
        (self.contains)(dtu)
    }
    fn lower_bound(&self, from: Dtu) -> LowerBound {
        match &self.lower_bound {
            Some(lower_bound) => lower_bound(from),
            None => LowerBound::Unknown,
        }
    }
}

impl Functional {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(Dtu) -> bool + Send + Sync + 'static,
    {
        Self {
            contains: Arc::new(f),
            lower_bound: None,
        }
    }
    pub fn with_lower_bound<F>(mut self, f: F) -> Self
    where
        F: Fn(Dtu) -> LowerBound + Send + Sync + 'static,
    {
        self.lower_bound = Some(Arc::new(f));
        self
    }
}

/// A finite set of time points, kept sorted so that the lower bound is a `O(log n)` lookup.
#[derive(Debug, Clone, Default)]
pub struct Discrete {
    pub values: BTreeSet<Dtu>,
}

impl Discrete {
    pub fn new<I: IntoIterator<Item = Dtu>>(values: I) -> Self {
        Self {
            values: values.into_iter().collect(),
        }
    }
}

impl<I: IntoIterator<Item = Dtu>> From<I> for Discrete {
    fn from(values: I) -> Self {
        Self::new(values)
    }
}

impl TimeSet for Discrete {
    fn contains(&self, dtu: Dtu) -> bool {
        self.values.contains(&dtu)
    }
    fn lower_bound(&self, from: Dtu) -> LowerBound {
        // the values are sorted, the first one at or after `from` is exactly the tightest bound
        match self.values.range(from..).next() {
            Some(next) => LowerBound::At(*next),
            None => LowerBound::Never,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RangeBound {
    pub value: Dtu,
    pub inclusive: bool,
}

impl RangeBound {
    pub fn inclusive(value: Dtu) -> Self {
        Self {
            value,
            inclusive: true,
        }
    }
    pub fn exclusive(value: Dtu) -> Self {
        Self {
            value,
            inclusive: false,
        }
    }
    /// the earliest time point allowed by this lower bound
    fn first_allowed(&self) -> Option<Dtu> {
        if self.inclusive {
            Some(self.value)
        } else {
            succ_dtu(self.value)
        }
    }
    fn allows_start(&self, dtu: Dtu) -> bool {
        if self.inclusive {
            dtu >= self.value
        } else {
            dtu > self.value
        }
    }
    fn allows_end(&self, dtu: Dtu) -> bool {
        if self.inclusive {
            dtu <= self.value
        } else {
            dtu < self.value
        }
    }
}

/// A continuous interval of time, at least one side must be bounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Range {
    pub from: Option<RangeBound>,
    pub to: Option<RangeBound>,
}

impl Range {
    /// create a range, both sides are optional but at least one of them is required
    pub fn new(from: Option<RangeBound>, to: Option<RangeBound>) -> Result<Self, InvalidRange> {
        match (from, to) {
            (None, None) => Err(InvalidRange::Unbounded),
            (Some(from), Some(to))
                if from.value > to.value
                    || (from.value == to.value && !(from.inclusive && to.inclusive)) =>
            {
                Err(InvalidRange::Empty { from, to })
            }
            (from, to) => Ok(Self { from, to }),
        }
    }
    /// `[from, +∞)`
    pub fn after(from: Dtu) -> Self {
        Self {
            from: Some(RangeBound::inclusive(from)),
            to: None,
        }
    }
    /// `(-∞, to)`
    pub fn before(to: Dtu) -> Self {
        Self {
            from: None,
            to: Some(RangeBound::exclusive(to)),
        }
    }
    /// `[from, to)`
    pub fn between(from: Dtu, to: Dtu) -> Result<Self, InvalidRange> {
        Self::new(
            Some(RangeBound::inclusive(from)),
            Some(RangeBound::exclusive(to)),
        )
    }
}

impl TimeSet for Range {
    fn contains(&self, dtu: Dtu) -> bool {
        self.from.is_none_or(|from| from.allows_start(dtu))
            && self.to.is_none_or(|to| to.allows_end(dtu))
    }
    fn lower_bound(&self, from: Dtu) -> LowerBound {
        // already past the end of the range
        if self.to.is_some_and(|to| !to.allows_end(from)) {
            return LowerBound::Never;
        }
        match self.from {
            // not reached the start of the range yet, jump right to it
            Some(start) if !start.allows_start(from) => match start.first_allowed() {
                Some(first) => LowerBound::At(first),
                None => LowerBound::Never,
            },
            _ => LowerBound::At(from),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvalidRange {
    Empty { from: RangeBound, to: RangeBound },
    Unbounded,
}

impl std::fmt::Display for InvalidRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty { from, to } => write!(
                f,
                "empty range: from {} ({}) to {} ({})",
                from.value,
                if from.inclusive {
                    "inclusive"
                } else {
                    "exclusive"
                },
                to.value,
                if to.inclusive {
                    "inclusive"
                } else {
                    "exclusive"
                },
            ),
            Self::Unbounded => write!(f, "range must have at least one bound"),
        }
    }
}

impl std::error::Error for InvalidRange {}

/// A time set containing nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Empty;

impl TimeSet for Empty {
    fn contains(&self, _dtu: Dtu) -> bool {
        false
    }
    fn lower_bound(&self, _from: Dtu) -> LowerBound {
        LowerBound::Never
    }
}

/// A time set containing every time point.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Universal;

impl TimeSet for Universal {
    fn contains(&self, _dtu: Dtu) -> bool {
        true
    }
    fn lower_bound(&self, from: Dtu) -> LowerBound {
        LowerBound::At(from)
    }
}

/// The union of several time sets.
///
/// The overall start bound is computed once on construction, so that the common case
/// "we are still far before the first interesting time point" is answered without walking
/// through all the members.
pub struct Union {
    collections: Vec<DynTimeSet>,
    start: LowerBound,
}

impl Union {
    pub fn new(collections: Vec<DynTimeSet>) -> Self {
        let start = LowerBound::join_all(
            collections
                .iter()
                .map(|collection| collection.lower_bound(min_dtu())),
        );
        Self { collections, start }
    }
    pub fn collections(&self) -> &[DynTimeSet] {
        &self.collections
    }
    /// the cached lower bound of the whole union
    pub fn start(&self) -> LowerBound {
        self.start
    }
    pub fn push<T: TimeSet>(&mut self, set: T) {
        self.start = self.start.join(set.lower_bound(min_dtu()));
        self.collections.push(set.dyn_box());
    }
}

impl FromIterator<DynTimeSet> for Union {
    fn from_iter<I: IntoIterator<Item = DynTimeSet>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

impl TimeSet for Union {
    fn contains(&self, dtu: Dtu) -> bool {
        // nothing can be contained before the cached start
        if !before_start(self.start, dtu) {
            return false;
        }
        self.collections
            .iter()
            .any(|collection| collection.contains(dtu))
    }
    fn lower_bound(&self, from: Dtu) -> LowerBound {
        match self.start {
            // fast path: still before the first candidate of the whole union
            LowerBound::Never => LowerBound::Never,
            LowerBound::At(start) if from <= start => LowerBound::At(start),
            _ => LowerBound::join_all(
                self.collections
                    .iter()
                    .map(|collection| collection.lower_bound(from)),
            ),
        }
    }
}

/// The intersection of several time sets.
///
/// Like [`Union`], the overall start bound is cached on construction.
pub struct Intersection {
    collections: Vec<DynTimeSet>,
    start: LowerBound,
}

impl Intersection {
    pub fn new(collections: Vec<DynTimeSet>) -> Self {
        let start = LowerBound::meet_all(
            collections
                .iter()
                .map(|collection| collection.lower_bound(min_dtu())),
        );
        Self { collections, start }
    }
    pub fn collections(&self) -> &[DynTimeSet] {
        &self.collections
    }
    /// the cached lower bound of the whole intersection
    pub fn start(&self) -> LowerBound {
        self.start
    }
    pub fn push<T: TimeSet>(&mut self, set: T) {
        self.start = self.start.meet(set.lower_bound(min_dtu()));
        self.collections.push(set.dyn_box());
    }
}

impl FromIterator<DynTimeSet> for Intersection {
    fn from_iter<I: IntoIterator<Item = DynTimeSet>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

impl TimeSet for Intersection {
    fn contains(&self, dtu: Dtu) -> bool {
        if !before_start(self.start, dtu) {
            return false;
        }
        self.collections
            .iter()
            .all(|collection| collection.contains(dtu))
    }
    fn lower_bound(&self, from: Dtu) -> LowerBound {
        match self.start {
            LowerBound::Never => LowerBound::Never,
            LowerBound::At(start) if from <= start => LowerBound::At(start),
            _ => LowerBound::meet_all(
                self.collections
                    .iter()
                    .map(|collection| collection.lower_bound(from)),
            ),
        }
    }
}

/// whether `dtu` may be contained by a set whose cached start bound is `start`
fn before_start(start: LowerBound, dtu: Dtu) -> bool {
    match start {
        LowerBound::Never => false,
        LowerBound::At(start) => dtu >= start,
        LowerBound::Unknown => true,
    }
}

/// The difference `a - b`: contained by `a` but not by `b`.
pub struct Difference<T: TimeSet = DynTimeSet, U: TimeSet = DynTimeSet> {
    pub a: T,
    pub b: U,
}

impl<T: TimeSet, U: TimeSet> Difference<T, U> {
    pub fn new(a: T, b: U) -> Self {
        Self { a, b }
    }
}

impl<T: TimeSet, U: TimeSet> TimeSet for Difference<T, U> {
    fn contains(&self, dtu: Dtu) -> bool {
        self.a.contains(dtu) && !self.b.contains(dtu)
    }
    fn lower_bound(&self, from: Dtu) -> LowerBound {
        // `a - b` is a subset of `a`, so any bound of `a` is sound here.
        // we cannot say more without knowing where the intervals of `b` end.
        self.a.lower_bound(from)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, Utc};

    use super::*;

    #[test]
    fn test_range_lower_bound() {
        let now = Utc::now();
        let later = now + TimeDelta::days(365);
        let range = Range::after(later);
        assert!(!range.contains(now));
        assert_eq!(range.lower_bound(now), LowerBound::At(later));
        assert_eq!(range.lower_bound(later), LowerBound::At(later));

        let range = Range::before(now);
        assert_eq!(range.lower_bound(now), LowerBound::Never);
        assert_eq!(range.lower_bound(later), LowerBound::Never);

        let range = Range::between(now, later).expect("valid range");
        assert!(range.contains(now));
        assert_eq!(range.lower_bound(min_dtu()), LowerBound::At(now));
        assert_eq!(range.lower_bound(later), LowerBound::Never);

        assert_eq!(Range::new(None, None), Err(InvalidRange::Unbounded));
        assert!(matches!(
            Range::between(later, now),
            Err(InvalidRange::Empty { .. })
        ));
    }

    #[test]
    fn test_discrete_lower_bound() {
        let now = Utc::now();
        let set = Discrete::new([now + TimeDelta::seconds(10), now + TimeDelta::seconds(5)]);
        assert_eq!(
            set.lower_bound(now),
            LowerBound::At(now + TimeDelta::seconds(5))
        );
        assert_eq!(
            set.lower_bound(now + TimeDelta::seconds(6)),
            LowerBound::At(now + TimeDelta::seconds(10))
        );
        assert_eq!(
            set.lower_bound(now + TimeDelta::seconds(11)),
            LowerBound::Never
        );
    }

    #[test]
    fn test_union_and_intersection_bounds() {
        let now = Utc::now();
        let a = Range::between(now + TimeDelta::days(1), now + TimeDelta::days(2)).expect("valid");
        let b =
            Range::between(now + TimeDelta::days(10), now + TimeDelta::days(11)).expect("valid");
        let union = Union::new(vec![a.dyn_box(), b.dyn_box()]);
        assert_eq!(union.start(), LowerBound::At(now + TimeDelta::days(1)));
        assert_eq!(
            union.lower_bound(now),
            LowerBound::At(now + TimeDelta::days(1))
        );
        assert_eq!(
            union.lower_bound(now + TimeDelta::days(3)),
            LowerBound::At(now + TimeDelta::days(10))
        );
        assert_eq!(
            union.lower_bound(now + TimeDelta::days(12)),
            LowerBound::Never
        );

        let intersection = Intersection::new(vec![
            Range::after(now + TimeDelta::days(1)).dyn_box(),
            Range::after(now + TimeDelta::days(10)).dyn_box(),
        ]);
        assert_eq!(
            intersection.lower_bound(now),
            LowerBound::At(now + TimeDelta::days(10))
        );
        assert!(!intersection.contains(now + TimeDelta::days(5)));
        assert!(intersection.contains(now + TimeDelta::days(11)));

        // unknown members poison the union bound but not the intersection one
        let with_unknown = Union::new(vec![a.dyn_box(), Functional::new(|_| false).dyn_box()]);
        assert_eq!(with_unknown.lower_bound(now), LowerBound::Unknown);
        let with_unknown =
            Intersection::new(vec![a.dyn_box(), Functional::new(|_| true).dyn_box()]);
        assert_eq!(
            with_unknown.lower_bound(now),
            LowerBound::At(now + TimeDelta::days(1))
        );
    }

    #[test]
    fn test_difference() {
        let now = Utc::now();
        let set = Range::after(now).difference(Discrete::new([now + TimeDelta::seconds(1)]));
        assert!(set.contains(now));
        assert!(!set.contains(now + TimeDelta::seconds(1)));
        assert_eq!(set.lower_bound(min_dtu()), LowerBound::At(now));
    }
}
