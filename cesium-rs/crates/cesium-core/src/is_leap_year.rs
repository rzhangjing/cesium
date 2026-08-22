//! Ported from packages/engine/Source/Core/isLeapYear.js

use crate::developer_error::throw_developer_error;

/// Determines if a given date is a leap year.
///
/// Port of CesiumJS `isLeapYear(year)`.
///
/// # Panics
/// In debug builds, panics with `DeveloperError` when `year` is NaN.
///
/// # Example
/// ```
/// # use cesium_core::is_leap_year::is_leap_year;
/// let leap_year = is_leap_year(2000.0); // true
/// assert!(leap_year);
/// ```
#[must_use]
pub fn is_leap_year(year: f64) -> bool {
    // >>includeStart('debug', pragmas.debug)
    if cfg!(debug_assertions) && year.is_nan() {
        throw_developer_error("year is required and must be a number.");
    }
    // >>includeEnd('debug')

    (year % 4.0 == 0.0 && year % 100.0 != 0.0) || year % 400.0 == 0.0
}
