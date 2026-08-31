//! Ported from `packages/engine/Source/Core/Iso8601.js`.
//!
//! Constants related to ISO 8601 support.

use crate::julian_date::JulianDate;
use crate::time_interval::TimeInterval;
use std::sync::OnceLock;

static MINIMUM_VALUE: OnceLock<JulianDate> = OnceLock::new();
static MAXIMUM_VALUE: OnceLock<JulianDate> = OnceLock::new();
static MAXIMUM_INTERVAL: OnceLock<TimeInterval> = OnceLock::new();

/// Constants related to ISO 8601 support.
pub struct Iso8601;

impl Iso8601 {
    /// A `JulianDate` representing the earliest time representable by an ISO 8601 date.
    /// Equivalent to '0000-01-01T00:00:00Z'.
    pub fn minimum_value() -> &'static JulianDate {
        MINIMUM_VALUE.get_or_init(|| {
            JulianDate::from_iso8601("0000-01-01T00:00:00Z")
                .unwrap_or_else(JulianDate::default_date)
        })
    }

    /// A `JulianDate` representing the latest time representable by an ISO 8601 date.
    /// Equivalent to '9999-12-31T24:00:00Z'.
    pub fn maximum_value() -> &'static JulianDate {
        MAXIMUM_VALUE.get_or_init(|| {
            JulianDate::from_iso8601("9999-12-31T24:00:00Z")
                .unwrap_or_else(JulianDate::default_date)
        })
    }

    /// A `TimeInterval` representing the largest interval representable by an ISO 8601 interval.
    /// Equivalent to '0000-01-01T00:00:00Z/9999-12-31T24:00:00Z'.
    pub fn maximum_interval() -> &'static TimeInterval {
        MAXIMUM_INTERVAL.get_or_init(|| TimeInterval {
            start: Self::minimum_value().clone(),
            stop: Self::maximum_value().clone(),
            is_start_included: true,
            is_stop_included: true,
            data: None,
        })
    }
}
