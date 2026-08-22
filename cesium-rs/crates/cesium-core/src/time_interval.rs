//! Ported from `packages/engine/Source/Core/TimeInterval.js`.
//!
//! An interval defined by a start and a stop time.

use crate::julian_date::JulianDate;

/// An interval defined by a start and a stop time; optionally including those
/// times as part of the interval.
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
        }
    }

    /// Returns true if this interval is empty.
    pub fn is_empty(&self) -> bool {
        let cmp = JulianDate::compare(&self.stop, &self.start);
        cmp < 0 || (cmp == 0 && (!self.is_start_included || !self.is_stop_included))
    }

    /// Creates a TimeInterval from an ISO 8601 interval string (e.g. "2000/2010").
    pub fn from_iso8601(
        iso8601: &str,
        is_start_included: Option<bool>,
        is_stop_included: Option<bool>,
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

    /// Computes the intersection of two intervals.
    pub fn intersect(left: &TimeInterval, right: &TimeInterval) -> Self {
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
        }
    }

    /// Compares two instances for equality.
    pub fn equals(left: &TimeInterval, right: &TimeInterval) -> bool {
        (left.is_empty() && right.is_empty())
            || (left.is_start_included == right.is_start_included
                && left.is_stop_included == right.is_stop_included
                && JulianDate::equals(&left.start, &right.start)
                && JulianDate::equals(&left.stop, &right.stop))
    }

    /// Compares two instances within epsilon.
    pub fn equals_epsilon(left: &TimeInterval, right: &TimeInterval, epsilon: f64) -> bool {
        (left.is_empty() && right.is_empty())
            || (left.is_start_included == right.is_start_included
                && left.is_stop_included == right.is_stop_included
                && JulianDate::equals_epsilon(&left.start, &right.start, epsilon)
                && JulianDate::equals_epsilon(&left.stop, &right.stop, epsilon))
    }

    /// An immutable empty interval.
    pub fn empty() -> Self {
        Self {
            start: JulianDate::default_date(),
            stop: JulianDate::default_date(),
            is_start_included: false,
            is_stop_included: false,
        }
    }
}

impl std::fmt::Display for TimeInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_iso8601(None))
    }
}
