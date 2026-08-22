//! Ported from packages/engine/Source/Core/TimeStandard.js

/// Enumerates the time standards used by [`JulianDate`](crate::julian_date::JulianDate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TimeStandard {
    /// Coordinated Universal Time.
    UTC = 0,
    /// International Atomic Time.
    TAI = 1,
}
