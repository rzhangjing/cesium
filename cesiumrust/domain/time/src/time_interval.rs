//! TimeInterval - time interval with start/stop inclusion flags.
//! Maps to CesiumJS `Core/TimeInterval.js`

use crate::julian_date::JulianDate;
use serde::{Deserialize, Serialize};

/// An interval defined by a start and stop time, optionally including those times.
/// Maps to CesiumJS `TimeInterval`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeInterval {
    /// The start time of the interval.
    pub start: JulianDate,
    /// The stop time of the interval.
    pub stop: JulianDate,
    /// Whether the start time is included in the interval.
    pub is_start_included: bool,
    /// Whether the stop time is included in the interval.
    pub is_stop_included: bool,
}

impl TimeInterval {
    /// Creates a new TimeInterval.
    pub fn new(
        start: JulianDate,
        stop: JulianDate,
        is_start_included: bool,
        is_stop_included: bool,
    ) -> Self {
        Self {
            start,
            stop,
            is_start_included,
            is_stop_included,
        }
    }

    /// Returns true if this interval is empty.
    /// Maps to `TimeInterval.isEmpty`
    pub fn is_empty(&self) -> bool {
        let cmp = self.stop.cmp(&self.start);
        cmp == std::cmp::Ordering::Less
            || (cmp == std::cmp::Ordering::Equal
                && (!self.is_start_included || !self.is_stop_included))
    }

    /// Returns true if the interval contains the given time.
    /// Maps to `TimeInterval.contains`
    pub fn contains(&self, time: &JulianDate) -> bool {
        if self.is_empty() {
            return false;
        }

        let start_cmp = time.cmp(&self.start);
        let stop_cmp = time.cmp(&self.stop);

        let after_start = if self.is_start_included {
            start_cmp != std::cmp::Ordering::Less
        } else {
            start_cmp == std::cmp::Ordering::Greater
        };

        let before_stop = if self.is_stop_included {
            stop_cmp != std::cmp::Ordering::Greater
        } else {
            stop_cmp == std::cmp::Ordering::Less
        };

        after_start && before_stop
    }

    /// Computes the intersection of two intervals.
    /// Maps to `TimeInterval.intersect`
    pub fn intersect(&self, other: &Self) -> Self {
        // Determine the later start
        let (start, is_start_included) = if self.start > other.start {
            (self.start, self.is_start_included)
        } else if other.start > self.start {
            (other.start, other.is_start_included)
        } else {
            (self.start, self.is_start_included && other.is_start_included)
        };

        // Determine the earlier stop
        let (stop, is_stop_included) = if self.stop < other.stop {
            (self.stop, self.is_stop_included)
        } else if other.stop < self.stop {
            (other.stop, other.is_stop_included)
        } else {
            (self.stop, self.is_stop_included && other.is_stop_included)
        };

        let result = Self::new(start, stop, is_start_included, is_stop_included);
        if result.is_empty() {
            Self::EMPTY
        } else {
            result
        }
    }

    /// An empty interval.
    pub const EMPTY: Self = Self {
        start: JulianDate { day_number: 0, seconds_of_day: 0.0 },
        stop: JulianDate { day_number: 0, seconds_of_day: 0.0 },
        is_start_included: false,
        is_stop_included: false,
    };

    /// Creates a TimeInterval from an ISO 8601 interval string ("start/stop").
    /// Maps to `TimeInterval.fromIso8601`
    pub fn from_iso8601(
        iso8601: &str,
        is_start_included: bool,
        is_stop_included: bool,
    ) -> Option<Self> {
        let parts: Vec<&str> = iso8601.split('/').collect();
        if parts.len() != 2 {
            return None;
        }
        let start = JulianDate::from_iso8601(parts[0])?;
        let stop = JulianDate::from_iso8601(parts[1])?;
        Some(Self::new(start, stop, is_start_included, is_stop_included))
    }

    /// Formats this interval as an ISO 8601 interval string.
    /// Maps to `TimeInterval.toIso8601`
    pub fn to_iso8601(&self) -> String {
        format!("{}/{}", self.start.to_iso8601(), self.stop.to_iso8601())
    }

    /// Formats this interval as an ISO 8601 interval string with specified precision.
    pub fn to_iso8601_with_precision(&self, precision: Option<usize>) -> String {
        format!(
            "{}/{}",
            self.start.to_iso8601_with_precision(precision),
            self.stop.to_iso8601_with_precision(precision)
        )
    }

    /// Compares two intervals for equality within an epsilon (seconds).
    /// Maps to `TimeInterval.equalsEpsilon`
    pub fn equals_epsilon(&self, other: &Self, epsilon: f64) -> bool {
        self.start.equals_epsilon(&other.start, epsilon)
            && self.stop.equals_epsilon(&other.stop, epsilon)
            && self.is_start_included == other.is_start_included
            && self.is_stop_included == other.is_stop_included
    }

    /// The duration of the interval in seconds.
    pub fn duration_seconds(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            self.stop.seconds_difference(&self.start)
        }
    }
}

impl Default for TimeInterval {
    fn default() -> Self {
        Self {
            start: JulianDate::default(),
            stop: JulianDate::default(),
            is_start_included: true,
            is_stop_included: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains() {
        let start = JulianDate::from_date_components(2000, 1, 1, 0, 0, 0, 0.0);
        let stop = JulianDate::from_date_components(2000, 1, 2, 0, 0, 0, 0.0);
        let interval = TimeInterval::new(start, stop, true, true);

        let inside = JulianDate::from_date_components(2000, 1, 1, 12, 0, 0, 0.0);
        assert!(interval.contains(&inside));
        assert!(interval.contains(&start));
        assert!(interval.contains(&stop));

        let outside = JulianDate::from_date_components(2000, 1, 3, 0, 0, 0, 0.0);
        assert!(!interval.contains(&outside));
    }

    #[test]
    fn test_exclusive_bounds() {
        let start = JulianDate::from_date_components(2000, 1, 1, 0, 0, 0, 0.0);
        let stop = JulianDate::from_date_components(2000, 1, 2, 0, 0, 0, 0.0);
        let interval = TimeInterval::new(start, stop, false, false);

        assert!(!interval.contains(&start));
        assert!(!interval.contains(&stop));

        let inside = JulianDate::from_date_components(2000, 1, 1, 12, 0, 0, 0.0);
        assert!(interval.contains(&inside));
    }

    #[test]
    fn test_is_empty() {
        let start = JulianDate::from_date_components(2000, 1, 1, 0, 0, 0, 0.0);
        let stop = JulianDate::from_date_components(2000, 1, 2, 0, 0, 0, 0.0);

        let normal = TimeInterval::new(start, stop, true, true);
        assert!(!normal.is_empty());

        let inverted = TimeInterval::new(stop, start, true, true);
        assert!(inverted.is_empty());

        let point_exclusive = TimeInterval::new(start, start, false, true);
        assert!(point_exclusive.is_empty());
    }

    #[test]
    fn test_intersect() {
        let start1 = JulianDate::from_date_components(2000, 1, 1, 0, 0, 0, 0.0);
        let stop1 = JulianDate::from_date_components(2000, 1, 10, 0, 0, 0, 0.0);
        let interval1 = TimeInterval::new(start1, stop1, true, true);

        let start2 = JulianDate::from_date_components(2000, 1, 5, 0, 0, 0, 0.0);
        let stop2 = JulianDate::from_date_components(2000, 1, 15, 0, 0, 0, 0.0);
        let interval2 = TimeInterval::new(start2, stop2, true, true);

        let intersection = interval1.intersect(&interval2);
        assert_eq!(intersection.start, start2);
        assert_eq!(intersection.stop, stop1);
    }

    #[test]
    fn test_no_intersection() {
        let start1 = JulianDate::from_date_components(2000, 1, 1, 0, 0, 0, 0.0);
        let stop1 = JulianDate::from_date_components(2000, 1, 5, 0, 0, 0, 0.0);
        let interval1 = TimeInterval::new(start1, stop1, true, true);

        let start2 = JulianDate::from_date_components(2000, 1, 10, 0, 0, 0, 0.0);
        let stop2 = JulianDate::from_date_components(2000, 1, 15, 0, 0, 0, 0.0);
        let interval2 = TimeInterval::new(start2, stop2, true, true);

        assert!(interval1.intersect(&interval2).is_empty());
    }
}
