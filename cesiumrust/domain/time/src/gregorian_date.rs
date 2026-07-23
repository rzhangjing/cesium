//! GregorianDate - calendar date representation.
//! Maps to CesiumJS `Core/GregorianDate.js`

use serde::{Deserialize, Serialize};

/// A calendar date in the Gregorian calendar.
/// Maps to CesiumJS `GregorianDate`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GregorianDate {
    /// The year.
    pub year: i32,
    /// The month (1-12).
    pub month: u32,
    /// The day of the month (1-31).
    pub day: u32,
    /// The hour (0-23).
    pub hour: u32,
    /// The minute (0-59).
    pub minute: u32,
    /// The second (0-60, 60 for leap seconds).
    pub second: u32,
    /// The millisecond (0-999.999...).
    pub millisecond: f64,
    /// Whether this date is during a leap second.
    pub is_leap_second: bool,
}

impl GregorianDate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
        millisecond: f64,
        is_leap_second: bool,
    ) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
            is_leap_second,
        }
    }
}

impl Default for GregorianDate {
    fn default() -> Self {
        Self {
            year: 2000,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            millisecond: 0.0,
            is_leap_second: false,
        }
    }
}
