//! Ported from packages/engine/Source/Core/TimeConstants.js

/// The number of seconds in one millisecond.
pub const SECONDS_PER_MILLISECOND: f64 = 0.001;

/// The number of seconds in one minute.
pub const SECONDS_PER_MINUTE: f64 = 60.0;

/// The number of minutes in one hour.
pub const MINUTES_PER_HOUR: f64 = 60.0;

/// The number of hours in one day.
pub const HOURS_PER_DAY: f64 = 24.0;

/// The number of seconds in one hour.
pub const SECONDS_PER_HOUR: f64 = 3600.0;

/// The number of minutes in one day.
pub const MINUTES_PER_DAY: f64 = 1440.0;

/// The number of seconds in one day, ignoring leap seconds.
pub const SECONDS_PER_DAY: f64 = 86400.0;

/// The number of days in one Julian century.
pub const DAYS_PER_JULIAN_CENTURY: f64 = 36525.0;

/// One trillionth of a second.
pub const PICOSECOND: f64 = 0.000000001;

/// The number of days to subtract from a Julian date to determine the
/// modified Julian date (days since midnight on November 17, 1858).
pub const MODIFIED_JULIAN_DATE_DIFFERENCE: f64 = 2400000.5;
