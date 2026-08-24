//! Ported from `packages/engine/Source/DataSources/DataSourceClock.js`.
//!
//! Defines the clock settings for a data source.

use cesium_core::clock_range::ClockRange;
use cesium_core::clock_step::ClockStep;
use cesium_core::julian_date::JulianDate;

/// Defines the clock settings for a data source.
#[derive(Debug, Clone)]
pub struct DataSourceClock {
    /// The start time of the clock interval.
    pub start_time: JulianDate,
    /// The stop time of the clock interval.
    pub stop_time: JulianDate,
    /// The current time.
    pub current_time: JulianDate,
    /// The behavior when `start_time` or `stop_time` is reached.
    pub clock_range: ClockRange,
    /// How the clock advances on each tick.
    pub clock_step: ClockStep,
    /// The multiplier (speed) applied when ticking.
    pub multiplier: f64,
}

impl DataSourceClock {
    /// Creates a new data source clock with default values.
    ///
    /// In CesiumJS the default start/stop/current times are `new JulianDate()`
    /// (the current system time); this port uses the crate's deterministic
    /// default date instead.
    pub fn new() -> Self {
        Self {
            start_time: JulianDate::default_date(),
            stop_time: JulianDate::default_date(),
            current_time: JulianDate::default_date(),
            clock_range: ClockRange::Unbounded,
            clock_step: ClockStep::SystemClockMultiplier,
            multiplier: 1.0,
        }
    }

    /// Returns whether this clock is equivalent to the provided clock.
    ///
    /// Port of `DataSourceClock.equals(right)`: an undefined `right` is never
    /// equal to a defined clock.
    pub fn equals(&self, other: Option<&DataSourceClock>) -> bool {
        let Some(other) = other else {
            return false;
        };
        JulianDate::equals(&self.start_time, &other.start_time)
            && JulianDate::equals(&self.stop_time, &other.stop_time)
            && JulianDate::equals(&self.current_time, &other.current_time)
            && self.clock_range == other.clock_range
            && self.clock_step == other.clock_step
            && self.multiplier == other.multiplier
    }

    /// Returns an equivalent copy of this clock (mirror of
    /// `DataSourceClock.clone`).
    pub fn clone_clock(&self) -> DataSourceClock {
        self.clone()
    }

    /// Merges another clock's settings into this one: each defined field of
    /// `other` overwrites the corresponding field of this clock (mirror of
    /// `DataSourceClock.merge`).
    pub fn merge(&mut self, other: &DataSourceClock) {
        self.start_time = other.start_time.clone();
        self.stop_time = other.stop_time.clone();
        self.current_time = other.current_time.clone();
        self.clock_range = other.clock_range;
        self.clock_step = other.clock_step;
        self.multiplier = other.multiplier;
    }
}

impl Default for DataSourceClock {
    fn default() -> Self { Self::new() }
}
