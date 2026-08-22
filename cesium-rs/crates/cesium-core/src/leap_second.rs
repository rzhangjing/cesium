//! Ported from packages/engine/Source/Core/LeapSecond.js

use crate::julian_date::JulianDate;

/// Describes a single leap second.
///
/// A leap second is constructed from a [`JulianDate`] and a numerical offset
/// representing the number of seconds TAI is ahead of the UTC time standard.
#[derive(Debug, Clone)]
pub struct LeapSecond {
    /// The Julian date at which this leap second occurs.
    pub julian_date: JulianDate,
    /// The cumulative number of seconds that TAI is ahead of UTC at the
    /// provided date.
    pub offset: f64,
}

impl LeapSecond {
    /// Creates a new leap second.
    pub fn new(julian_date: JulianDate, offset: f64) -> Self {
        Self {
            julian_date,
            offset,
        }
    }
}
