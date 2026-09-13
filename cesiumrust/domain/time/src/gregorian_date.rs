//! GregorianDate - calendar date representation.
//! Maps to CesiumJS `Core/GregorianDate.js`

use serde::{Deserialize, Serialize};

/// Days in each month (non-leap year). Index 0 = January.
const DAYS_IN_MONTH: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Returns true if the given year is a leap year.
/// Maps to CesiumJS `isLeapYear`
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Returns the number of days in the given month (1-based) for the given year.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    if month == 2 && is_leap_year(year) {
        29
    } else {
        DAYS_IN_MONTH[(month - 1) as usize]
    }
}

/// A calendar date in the Gregorian calendar.
/// Maps to CesiumJS `GregorianDate`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GregorianDate {
    /// The year (1-9999).
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
    /// Creates a new GregorianDate with validation.
    /// Maps to CesiumJS `new GregorianDate(year, month, day, hour, minute, second, millisecond, isLeapSecond)`
    ///
    /// Validation is debug-only (matches CesiumJS DeveloperError behavior).
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
        // Debug-only validation (matches CesiumJS `//>>includeStart('debug')` blocks)
        debug_assert!(year >= 1 && year <= 9999, "Year must be in range [1, 9999], got {year}");
        debug_assert!(month >= 1 && month <= 12, "Month must be in range [1, 12], got {month}");
        debug_assert!(day >= 1 && day <= 31, "Day must be in range [1, 31], got {day}");
        debug_assert!(hour <= 23, "Hour must be in range [0, 23], got {hour}");
        debug_assert!(minute <= 59, "Minute must be in range [0, 59], got {minute}");
        let max_second = if is_leap_second { 60 } else { 59 };
        debug_assert!(second <= max_second, "Second must be in range [0, {max_second}], got {second}");
        debug_assert!(millisecond >= 0.0 && millisecond < 1000.0,
            "Millisecond must be in range [0, 1000), got {millisecond}");
        // Validate day is valid for the given month/year
        if month >= 1 && month <= 12 {
            let max_day = days_in_month(year, month);
            debug_assert!(day <= max_day,
                "Day {day} is invalid for year {year} month {month} (max {max_day})");
        }

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
    /// Constructs the minimum date (year 1, month 1, day 1, midnight).
    /// Maps to CesiumJS `new GregorianDate()` with all defaults.
    fn default() -> Self {
        Self {
            year: 1,
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
