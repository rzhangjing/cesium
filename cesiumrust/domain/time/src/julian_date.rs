//! JulianDate - astronomical Julian date representation.
//! Maps to CesiumJS `Core/JulianDate.js`
//!
//! Stores time as dayNumber + secondsOfDay (TAI standard internally).
//! The Julian date is the number of days since noon on January 1, -4712 (4713 BC).

use crate::gregorian_date::GregorianDate;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Time constants used for conversions.
/// Maps to CesiumJS `Core/TimeConstants.js`
pub mod constants {
    pub const SECONDS_PER_MILLISECOND: f64 = 0.001;
    pub const SECONDS_PER_MINUTE: f64 = 60.0;
    pub const MINUTES_PER_HOUR: f64 = 60.0;
    pub const HOURS_PER_DAY: f64 = 24.0;
    pub const SECONDS_PER_HOUR: f64 = 3600.0;
    pub const MINUTES_PER_DAY: f64 = 1440.0;
    pub const SECONDS_PER_DAY: f64 = 86400.0;
    pub const DAYS_PER_JULIAN_CENTURY: f64 = 36525.0;
    pub const MODIFIED_JULIAN_DATE_DIFFERENCE: f64 = 2400000.5;
}

use constants::*;

/// Represents an astronomical Julian date.
/// Maps to CesiumJS `JulianDate`
///
/// For increased precision, stores the whole number part of the date and the
/// seconds part in separate components. Internally stored in TAI.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct JulianDate {
    /// The number of whole days.
    pub day_number: i64,
    /// The number of seconds into the current day.
    pub seconds_of_day: f64,
}

impl JulianDate {
    /// Creates a new JulianDate from day number and seconds of day.
    /// Maps to `new JulianDate(julianDayNumber, secondsOfDay, timeStandard)`
    pub fn new(julian_day_number: f64, seconds_of_day: f64) -> Self {
        let whole_days = julian_day_number as i64;
        let sod = seconds_of_day + (julian_day_number - whole_days as f64) * SECONDS_PER_DAY;
        Self::set_components(whole_days, sod)
    }

    /// Creates a JulianDate from date components (UTC assumed, no leap second correction).
    /// Maps to `JulianDate.fromGregorianDate` / `computeJulianDateComponents`
    pub fn from_date_components(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
        millisecond: f64,
    ) -> Self {
        let (day_number, seconds_of_day) =
            compute_julian_date_components(year, month, day, hour, minute, second, millisecond);
        Self::set_components(day_number, seconds_of_day)
    }

    /// Creates a JulianDate from a GregorianDate.
    pub fn from_gregorian_date(date: &GregorianDate) -> Self {
        Self::from_date_components(
            date.year,
            date.month,
            date.day,
            date.hour,
            date.minute,
            date.second,
            date.millisecond,
        )
    }

    /// Creates a JulianDate representing the current system time.
    /// Maps to `JulianDate.now()`
    pub fn now() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let unix_seconds = now.as_secs_f64();
        // Unix epoch (1970-01-01) = Julian date 2440587.5
        let julian_days = unix_seconds / SECONDS_PER_DAY + 2440587.5;
        Self::new(julian_days, 0.0)
    }

    /// Creates a JulianDate from seconds since Unix epoch.
    pub fn from_unix_seconds(unix_seconds: f64) -> Self {
        let julian_days = unix_seconds / SECONDS_PER_DAY + 2440587.5;
        Self::new(julian_days, 0.0)
    }

    /// Converts to seconds since Unix epoch.
    pub fn to_unix_seconds(&self) -> f64 {
        (self.total_days() - 2440587.5) * SECONDS_PER_DAY
    }

    /// Normalizes day_number and seconds_of_day components.
    /// Maps to `setComponents`
    fn set_components(whole_days: i64, seconds_of_day: f64) -> Self {
        let extra_days = (seconds_of_day / SECONDS_PER_DAY) as i64;
        let mut day_number = whole_days + extra_days;
        let mut sod = seconds_of_day - SECONDS_PER_DAY * extra_days as f64;

        if sod < 0.0 {
            day_number -= 1;
            sod += SECONDS_PER_DAY;
        }

        Self {
            day_number,
            seconds_of_day: sod,
        }
    }

    /// Computes the total number of whole and fractional days.
    /// Maps to `JulianDate.totalDays`
    pub fn total_days(&self) -> f64 {
        self.day_number as f64 + self.seconds_of_day / SECONDS_PER_DAY
    }

    /// Computes the difference in seconds between two dates (left - right).
    /// Maps to `JulianDate.secondsDifference`
    pub fn seconds_difference(&self, other: &Self) -> f64 {
        let day_diff = (self.day_number - other.day_number) as f64 * SECONDS_PER_DAY;
        day_diff + (self.seconds_of_day - other.seconds_of_day)
    }

    /// Computes the difference in days between two dates (left - right).
    /// Maps to `JulianDate.daysDifference`
    pub fn days_difference(&self, other: &Self) -> f64 {
        let day_diff = (self.day_number - other.day_number) as f64;
        let second_diff = (self.seconds_of_day - other.seconds_of_day) / SECONDS_PER_DAY;
        day_diff + second_diff
    }

    /// Adds seconds to this date.
    /// Maps to `JulianDate.addSeconds`
    pub fn add_seconds(&self, seconds: f64) -> Self {
        Self::set_components(self.day_number, self.seconds_of_day + seconds)
    }

    /// Adds minutes to this date.
    /// Maps to `JulianDate.addMinutes`
    pub fn add_minutes(&self, minutes: f64) -> Self {
        Self::set_components(
            self.day_number,
            self.seconds_of_day + minutes * SECONDS_PER_MINUTE,
        )
    }

    /// Adds hours to this date.
    /// Maps to `JulianDate.addHours`
    pub fn add_hours(&self, hours: f64) -> Self {
        Self::set_components(
            self.day_number,
            self.seconds_of_day + hours * SECONDS_PER_HOUR,
        )
    }

    /// Adds days to this date.
    /// Maps to `JulianDate.addDays`
    pub fn add_days(&self, days: f64) -> Self {
        let extra_days = days as i64;
        let remaining_seconds = (days - extra_days as f64) * SECONDS_PER_DAY;
        Self::set_components(
            self.day_number + extra_days,
            self.seconds_of_day + remaining_seconds,
        )
    }

    /// Returns true if this date is before the other.
    /// Maps to `JulianDate.lessThan`
    pub fn less_than(&self, other: &Self) -> bool {
        self.day_number < other.day_number
            || (self.day_number == other.day_number
                && self.seconds_of_day < other.seconds_of_day)
    }

    /// Returns true if this date is after the other.
    /// Maps to `JulianDate.greaterThan`
    pub fn greater_than(&self, other: &Self) -> bool {
        self.day_number > other.day_number
            || (self.day_number == other.day_number
                && self.seconds_of_day > other.seconds_of_day)
    }

    /// Returns true if the two dates are within epsilon seconds of each other.
    /// Maps to `JulianDate.equalsEpsilon`
    pub fn equals_epsilon(&self, other: &Self, epsilon: f64) -> bool {
        self.seconds_difference(other).abs() <= epsilon
    }

    /// Converts to a GregorianDate.
    /// Maps to `JulianDate.toGregorianDate`
    pub fn to_gregorian_date(&self) -> GregorianDate {
        let mut julian_day_number = self.day_number;
        let seconds_of_day = self.seconds_of_day;

        if seconds_of_day >= 43200.0 {
            julian_day_number += 1;
        }

        // Algorithm from page 604 of the Explanatory Supplement to the
        // Astronomical Almanac (Seidelmann 1992).
        let l = julian_day_number + 68569;
        let n = (4 * l) / 146097;
        let l = l - (146097 * n + 3) / 4;
        let i = (4000 * (l + 1)) / 1461001;
        let l = l - (1461 * i) / 4 + 31;
        let j = (80 * l) / 2447;
        let day = (l - (2447 * j) / 80) as u32;
        let l = j / 11;
        let month = (j + 2 - 12 * l) as u32;
        let year = (100 * (n - 49) + i + l) as i32;

        let mut hour = (seconds_of_day / SECONDS_PER_HOUR) as u32;
        let mut remaining_seconds = seconds_of_day - hour as f64 * SECONDS_PER_HOUR;
        let minute = (remaining_seconds / SECONDS_PER_MINUTE) as u32;
        remaining_seconds -= minute as f64 * SECONDS_PER_MINUTE;
        let second = remaining_seconds as u32;
        let millisecond = (remaining_seconds - second as f64) / SECONDS_PER_MILLISECOND;

        // JulianDates are noon-based
        hour += 12;
        if hour > 23 {
            hour -= 24;
        }

        GregorianDate::new(year, month, day, hour, minute, second, millisecond, false)
    }

    /// Converts to ISO 8601 string representation.
    /// Maps to `JulianDate.toIso8601`
    pub fn to_iso8601(&self) -> String {
        let g = self.to_gregorian_date();
        if g.millisecond.abs() > 1e-10 {
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                g.year, g.month, g.day, g.hour, g.minute, g.second, g.millisecond as u32
            )
        } else {
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                g.year, g.month, g.day, g.hour, g.minute, g.second
            )
        }
    }

    /// The Julian date of the Unix epoch (1970-01-01T00:00:00Z).
    pub const UNIX_EPOCH: Self = Self {
        day_number: 2440588,
        seconds_of_day: 0.0, // noon-based, so midnight = 43200 seconds before noon
    };

    /// The Julian date of the J2000 epoch (2000-01-01T12:00:00 TAI).
    pub const J2000: Self = Self {
        day_number: 2451545,
        seconds_of_day: 0.0,
    };
}

impl PartialOrd for JulianDate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for JulianDate {}

impl Ord for JulianDate {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.day_number.cmp(&other.day_number) {
            Ordering::Equal => self.seconds_of_day.partial_cmp(&other.seconds_of_day).unwrap_or(Ordering::Equal),
            ord => ord,
        }
    }
}

impl Default for JulianDate {
    fn default() -> Self {
        Self {
            day_number: 0,
            seconds_of_day: 0.0,
        }
    }
}

/// Computes Julian date components from Gregorian date components.
/// Algorithm from page 604 of the Explanatory Supplement to the
/// Astronomical Almanac (Seidelmann 1992).
/// Maps to `computeJulianDateComponents`
fn compute_julian_date_components(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    millisecond: f64,
) -> (i64, f64) {
    let a = (month as i64 - 14) / 12;
    let b = year as i64 + 4800 + a;
    let mut day_number = (1461 * b / 4)
        + (367 * (month as i64 - 2 - 12 * a) / 12)
        - (3 * ((b + 100) / 100) / 4)
        + day as i64
        - 32075;

    // JulianDates are noon-based
    let mut hour = hour as f64 - 12.0;
    if hour < 0.0 {
        hour += 24.0;
    }

    let seconds_of_day = second as f64
        + hour * SECONDS_PER_HOUR
        + minute as f64 * SECONDS_PER_MINUTE
        + millisecond * SECONDS_PER_MILLISECOND;

    if seconds_of_day >= 43200.0 {
        day_number -= 1;
    }

    (day_number, seconds_of_day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_date_components_j2000() {
        // J2000 epoch: 2000-01-01T12:00:00
        let jd = JulianDate::from_date_components(2000, 1, 1, 12, 0, 0, 0.0);
        assert_eq!(jd.day_number, 2451545);
        assert!(jd.seconds_of_day.abs() < 1e-10);
    }

    #[test]
    fn test_from_date_components_unix_epoch() {
        // Unix epoch: 1970-01-01T00:00:00
        let jd = JulianDate::from_date_components(1970, 1, 1, 0, 0, 0, 0.0);
        // Unix epoch = JD 2440587.5, stored as day 2440587 + 43200 seconds
        assert_eq!(jd.day_number, 2440587);
        assert!((jd.seconds_of_day - 43200.0).abs() < 1e-10);
    }

    #[test]
    fn test_seconds_difference() {
        let jd1 = JulianDate::from_date_components(2000, 1, 1, 12, 0, 0, 0.0);
        let jd2 = JulianDate::from_date_components(2000, 1, 1, 12, 0, 1, 0.0);
        assert!((jd2.seconds_difference(&jd1) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_days_difference() {
        let jd1 = JulianDate::from_date_components(2000, 1, 1, 0, 0, 0, 0.0);
        let jd2 = JulianDate::from_date_components(2000, 1, 2, 0, 0, 0, 0.0);
        assert!((jd2.days_difference(&jd1) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_seconds() {
        let jd = JulianDate::from_date_components(2000, 1, 1, 12, 0, 0, 0.0);
        let jd2 = jd.add_seconds(3600.0);
        let g = jd2.to_gregorian_date();
        assert_eq!(g.hour, 13);
        assert_eq!(g.minute, 0);
    }

    #[test]
    fn test_add_days() {
        let jd = JulianDate::from_date_components(2000, 1, 1, 12, 0, 0, 0.0);
        let jd2 = jd.add_days(1.0);
        let g = jd2.to_gregorian_date();
        assert_eq!(g.day, 2);
    }

    #[test]
    fn test_to_gregorian_roundtrip() {
        let jd = JulianDate::from_date_components(2023, 6, 15, 14, 30, 45, 500.0);
        let g = jd.to_gregorian_date();
        assert_eq!(g.year, 2023);
        assert_eq!(g.month, 6);
        assert_eq!(g.day, 15);
        assert_eq!(g.hour, 14);
        assert_eq!(g.minute, 30);
        assert_eq!(g.second, 45);
        assert!((g.millisecond - 500.0).abs() < 1.0);
    }

    #[test]
    fn test_comparison() {
        let jd1 = JulianDate::from_date_components(2000, 1, 1, 0, 0, 0, 0.0);
        let jd2 = JulianDate::from_date_components(2000, 1, 2, 0, 0, 0, 0.0);
        assert!(jd1.less_than(&jd2));
        assert!(jd2.greater_than(&jd1));
        assert!(!jd1.equals_epsilon(&jd2, 0.001));
        assert!(jd1.equals_epsilon(&jd2, 86401.0));
    }

    #[test]
    fn test_total_days() {
        let jd = JulianDate::from_date_components(2000, 1, 1, 12, 0, 0, 0.0);
        assert!((jd.total_days() - 2451545.0).abs() < 1e-10);
    }

    #[test]
    fn test_unix_seconds_roundtrip() {
        let jd = JulianDate::from_date_components(2020, 6, 15, 12, 0, 0, 0.0);
        let unix = jd.to_unix_seconds();
        let jd2 = JulianDate::from_unix_seconds(unix);
        assert!(jd.equals_epsilon(&jd2, 0.001));
    }
}
