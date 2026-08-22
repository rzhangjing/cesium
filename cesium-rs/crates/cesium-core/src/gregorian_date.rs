//! Ported from packages/engine/Source/Core/GregorianDate.js

use crate::is_leap_year::is_leap_year;

const DAYS_IN_MONTH: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Represents a Gregorian date in a more precise format than the standard
/// date types. In addition to sub-millisecond precision, this object can also
/// represent leap seconds.
#[derive(Debug, Clone)]
pub struct GregorianDate {
    /// The year as a whole number.
    pub year: i32,
    /// The month as a whole number with range \[1, 12\].
    pub month: i32,
    /// The day of the month as a whole number starting at 1.
    pub day: i32,
    /// The hour as a whole number with range \[0, 23\].
    pub hour: i32,
    /// The minute of the hour as a whole number with range \[0, 59\].
    pub minute: i32,
    /// The second of the minute as a whole number with range \[0, 60\],
    /// with 60 representing a leap second.
    pub second: i32,
    /// The millisecond of the second as a floating point number with range
    /// \[0.0, 1000.0).
    pub millisecond: f64,
    /// Whether this time is during a leap second.
    pub is_leap_second: bool,
}

impl Default for GregorianDate {
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

impl GregorianDate {
    /// Creates a new GregorianDate with the given components.
    pub fn new(
        year: i32,
        month: i32,
        day: i32,
        hour: i32,
        minute: i32,
        second: i32,
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

    /// Validates the date components.
    pub fn validate(&self) -> bool {
        if self.year < 1 || self.year > 9999 {
            return false;
        }
        if self.month < 1 || self.month > 12 {
            return false;
        }
        if self.day < 1 || self.day > 31 {
            return false;
        }
        if self.hour < 0 || self.hour > 23 {
            return false;
        }
        if self.minute < 0 || self.minute > 59 {
            return false;
        }
        let max_second = if self.is_leap_second { 60 } else { 59 };
        if self.second < 0 || self.second > max_second {
            return false;
        }
        if self.millisecond < 0.0 || self.millisecond >= 1000.0 {
            return false;
        }

        // Validate day against month
        let days_in_month = if self.month == 2 && is_leap_year(self.year as f64) {
            29
        } else {
            DAYS_IN_MONTH[(self.month - 1) as usize]
        };
        if self.day > days_in_month {
            return false;
        }

        true
    }
}
