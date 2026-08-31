//! Ported from `packages/engine/Source/Core/TimeInterval.js`.
//!
//! An interval defined by a start and a stop time.
//!
//! DEVIATION (data payload): the JS `TimeInterval.data` accepts an arbitrary
//! object (compared by reference by default). The Rust port models it with
//! the [`IntervalData`] enum; [`IntervalData::object`] values carry a unique
//! identity so that the default (comparer-less) equality mirrors JS reference
//! equality.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::julian_date::JulianDate;

static NEXT_OBJECT_ID: AtomicU64 = AtomicU64::new(1);

/// Arbitrary data associated with a [`TimeInterval`] (port of the JS
/// `TimeInterval.data` member).
#[derive(Clone, Debug, PartialEq)]
pub enum IntervalData {
    /// A numeric payload (JS number; `===` compares by value).
    Number(f64),
    /// A string payload (JS string; `===` compares by value).
    Text(String),
    /// A boolean payload (JS boolean; `===` compares by value).
    Boolean(bool),
    /// An opaque object payload (JS object; `===` compares by reference).
    /// Each call to [`IntervalData::object`] mints a fresh identity, and
    /// `PartialEq` compares identities, mirroring JS reference equality.
    Object(u64),
}

impl IntervalData {
    /// Creates a fresh object payload with a unique identity (mirrors
    /// `new SomeObject()` in JS: two instances are never `===`).
    #[must_use]
    pub fn object() -> Self {
        IntervalData::Object(NEXT_OBJECT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// A function which compares the data of two intervals (port of the JS
/// `TimeInterval.DataComparer` callback).
pub type DataComparer<'a> = dyn Fn(Option<&IntervalData>, Option<&IntervalData>) -> bool + 'a;

/// A function which merges the data of two intervals (port of the JS
/// `TimeInterval.MergeCallback` callback).
pub type MergeCallback<'a> =
    dyn Fn(Option<&IntervalData>, Option<&IntervalData>) -> Option<IntervalData> + 'a;

/// An interval defined by a start and a stop time; optionally including those
/// times as part of the interval. Arbitrary data can optionally be associated
/// with each instance for use with `TimeIntervalCollection`.
#[derive(Clone, Debug)]
pub struct TimeInterval {
    /// The start time of the interval.
    pub start: JulianDate,
    /// The stop time of the interval.
    pub stop: JulianDate,
    /// Whether the start time is included.
    pub is_start_included: bool,
    /// Whether the stop time is included.
    pub is_stop_included: bool,
    /// The data associated with this interval (JS `data`).
    pub data: Option<IntervalData>,
}

impl TimeInterval {
    /// Creates a new TimeInterval.
    pub fn new(
        start: Option<JulianDate>,
        stop: Option<JulianDate>,
        is_start_included: Option<bool>,
        is_stop_included: Option<bool>,
    ) -> Self {
        Self {
            start: start.unwrap_or_else(JulianDate::default_date),
            stop: stop.unwrap_or_else(JulianDate::default_date),
            is_start_included: is_start_included.unwrap_or(true),
            is_stop_included: is_stop_included.unwrap_or(true),
            data: None,
        }
    }

    /// Creates a new TimeInterval with associated data (JS `options.data`).
    pub fn new_with_data(
        start: Option<JulianDate>,
        stop: Option<JulianDate>,
        is_start_included: Option<bool>,
        is_stop_included: Option<bool>,
        data: Option<IntervalData>,
    ) -> Self {
        Self {
            start: start.unwrap_or_else(JulianDate::default_date),
            stop: stop.unwrap_or_else(JulianDate::default_date),
            is_start_included: is_start_included.unwrap_or(true),
            is_stop_included: is_stop_included.unwrap_or(true),
            data,
        }
    }

    /// Returns true if this interval is empty.
    pub fn is_empty(&self) -> bool {
        let cmp = JulianDate::compare(&self.stop, &self.start);
        cmp < 0 || (cmp == 0 && (!self.is_start_included || !self.is_stop_included))
    }

    /// Creates a TimeInterval from an ISO 8601 interval string (e.g. "2000/2010").
    ///
    /// DEVIATION (error type): the JS `fromIso8601` throws a `DeveloperError`
    /// for a malformed interval; the Rust port returns `None` to keep the
    /// established call sites (including the diff harness) intact.
    pub fn from_iso8601(
        iso8601: &str,
        is_start_included: Option<bool>,
        is_stop_included: Option<bool>,
    ) -> Option<Self> {
        Self::from_iso8601_with_data(iso8601, is_start_included, is_stop_included, None)
    }

    /// Creates a TimeInterval from an ISO 8601 interval string, associating
    /// `data` with the result (JS `options.data`).
    pub fn from_iso8601_with_data(
        iso8601: &str,
        is_start_included: Option<bool>,
        is_stop_included: Option<bool>,
        data: Option<IntervalData>,
    ) -> Option<Self> {
        let dates: Vec<&str> = iso8601.split('/').collect();
        if dates.len() != 2 {
            return None;
        }
        let start = JulianDate::from_iso8601(dates[0])?;
        let stop = JulianDate::from_iso8601(dates[1])?;
        Some(Self {
            start,
            stop,
            is_start_included: is_start_included.unwrap_or(true),
            is_stop_included: is_stop_included.unwrap_or(true),
            data,
        })
    }

    /// Creates an ISO 8601 representation.
    pub fn to_iso8601(&self, precision: Option<usize>) -> String {
        format!(
            "{}/{}",
            self.start.to_iso8601(precision),
            self.stop.to_iso8601(precision)
        )
    }

    /// Checks if the interval contains the specified date.
    pub fn contains(&self, julian_date: &JulianDate) -> bool {
        if self.is_empty() {
            return false;
        }
        let start_cmp = JulianDate::compare(&self.start, julian_date);
        if start_cmp == 0 {
            return self.is_start_included;
        }
        let stop_cmp = JulianDate::compare(julian_date, &self.stop);
        if stop_cmp == 0 {
            return self.is_stop_included;
        }
        start_cmp < 0 && stop_cmp < 0
    }

    /// Computes the intersection of two intervals. The data of the left
    /// interval is kept (JS `intersect` without a merge callback).
    pub fn intersect(left: &TimeInterval, right: &TimeInterval) -> Self {
        Self::intersect_with_callback(left, right, None)
    }

    /// Computes the intersection of two intervals, optionally merging their
    /// data (port of `TimeInterval.intersect(left, right, result,
    /// mergeCallback)`).
    pub fn intersect_with_callback(
        left: &TimeInterval,
        right: &TimeInterval,
        merge_callback: Option<&MergeCallback>,
    ) -> Self {
        let left_start = &left.start;
        let left_stop = &left.stop;
        let right_start = &right.start;
        let right_stop = &right.stop;

        let intersects_start_right = JulianDate::greater_than_or_equals(right_start, left_start)
            && JulianDate::greater_than_or_equals(left_stop, right_start);
        let intersects_start_left = !intersects_start_right
            && JulianDate::less_than_or_equals(right_start, left_start)
            && JulianDate::less_than_or_equals(left_start, right_stop);

        if !intersects_start_right && !intersects_start_left {
            return Self::empty();
        }

        let left_less_than_right = JulianDate::less_than(left_stop, right_stop);

        let start = if intersects_start_right {
            right_start.clone()
        } else {
            left_start.clone()
        };
        let is_start_included = (left.is_start_included && right.is_start_included)
            || (!JulianDate::equals(right_start, left_start)
                && ((intersects_start_right && right.is_start_included)
                    || (intersects_start_left && left.is_start_included)));
        let stop = if left_less_than_right {
            left_stop.clone()
        } else {
            right_stop.clone()
        };
        let is_stop_included = if left_less_than_right {
            left.is_stop_included
        } else {
            (left.is_stop_included && right.is_stop_included)
                || (!JulianDate::equals(right_stop, left_stop) && right.is_stop_included)
        };

        Self {
            start,
            stop,
            is_start_included,
            is_stop_included,
            data: match merge_callback {
                Some(merge_callback) => merge_callback(left.data.as_ref(), right.data.as_ref()),
                None => left.data.clone(),
            },
        }
    }

    /// Compares two instances for equality (JS `===` semantics on `data`).
    pub fn equals(left: &TimeInterval, right: &TimeInterval) -> bool {
        Self::equals_with(left, right, None)
    }

    /// Compares two instances for equality, using `data_comparer` for the
    /// data when provided (port of `TimeInterval.equals(left, right,
    /// dataComparer)`).
    pub fn equals_with(
        left: &TimeInterval,
        right: &TimeInterval,
        data_comparer: Option<&DataComparer>,
    ) -> bool {
        (left.is_empty() && right.is_empty())
            || (left.is_start_included == right.is_start_included
                && left.is_stop_included == right.is_stop_included
                && JulianDate::equals(&left.start, &right.start)
                && JulianDate::equals(&left.stop, &right.stop)
                && Self::data_equals(
                    left.data.as_ref(),
                    right.data.as_ref(),
                    data_comparer,
                ))
    }

    /// Compares two instances within epsilon (JS `===` semantics on `data`).
    pub fn equals_epsilon(left: &TimeInterval, right: &TimeInterval, epsilon: f64) -> bool {
        Self::equals_epsilon_with(left, right, epsilon, None)
    }

    /// Compares two instances within epsilon, using `data_comparer` for the
    /// data when provided (port of `TimeInterval.equalsEpsilon(left, right,
    /// epsilon, dataComparer)`).
    pub fn equals_epsilon_with(
        left: &TimeInterval,
        right: &TimeInterval,
        epsilon: f64,
        data_comparer: Option<&DataComparer>,
    ) -> bool {
        (left.is_empty() && right.is_empty())
            || (left.is_start_included == right.is_start_included
                && left.is_stop_included == right.is_stop_included
                && JulianDate::equals_epsilon(&left.start, &right.start, epsilon)
                && JulianDate::equals_epsilon(&left.stop, &right.stop, epsilon)
                && Self::data_equals(
                    left.data.as_ref(),
                    right.data.as_ref(),
                    data_comparer,
                ))
    }

    /// Port of the JS `left.data === right.data || (defined(dataComparer) &&
    /// dataComparer(left.data, right.data))` condition.
    fn data_equals(
        left: Option<&IntervalData>,
        right: Option<&IntervalData>,
        data_comparer: Option<&DataComparer>,
    ) -> bool {
        let reference_equal = match (left, right) {
            (None, None) => true,
            (Some(left), Some(right)) => left == right,
            _ => false,
        };
        reference_equal
            || data_comparer.is_some_and(|data_comparer| data_comparer(left, right))
    }

    /// An immutable empty interval (port of the `TimeInterval.EMPTY`
    /// constant).
    pub fn empty() -> Self {
        Self {
            start: JulianDate::default_date(),
            stop: JulianDate::default_date(),
            is_start_included: false,
            is_stop_included: false,
            data: None,
        }
    }
}

impl std::fmt::Display for TimeInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_iso8601(None))
    }
}
