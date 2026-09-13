//! DataSourceClock - clock settings associated with a DataSource.
//!
//! Maps to CesiumJS `DataSources/DataSourceClock.js`

use cesium_time::{Clock, ClockOptions, ClockRange, ClockStep, JulianDate};

/// Clock settings associated with a DataSource. Provides merge/clone/getValue
/// semantics matching CesiumJS DataSourceClock.
///
/// Maps to CesiumJS `DataSources/DataSourceClock.js`
#[derive(Debug, Clone, Default)]
pub struct DataSourceClock {
    /// The start time of the clock.
    pub start_time: Option<JulianDate>,
    /// The stop time of the clock.
    pub stop_time: Option<JulianDate>,
    /// The current time.
    pub current_time: Option<JulianDate>,
    /// Determines how the clock behaves at start/stop boundaries.
    pub clock_range: Option<ClockRange>,
    /// Determines how time advances per tick.
    pub clock_step: Option<ClockStep>,
    /// The multiplier for time advancement.
    pub multiplier: Option<f64>,
}

impl DataSourceClock {
    /// Creates a new DataSourceClock with all fields unset.
    pub fn new() -> Self {
        Self::default()
    }

    /// Merges unassigned properties from `source` into this clock.
    /// Properties that are already assigned are not overwritten.
    ///
    /// Maps to `DataSourceClock.prototype.merge`
    pub fn merge(&mut self, source: &DataSourceClock) {
        if self.start_time.is_none() {
            self.start_time = source.start_time;
        }
        if self.stop_time.is_none() {
            self.stop_time = source.stop_time;
        }
        if self.current_time.is_none() {
            self.current_time = source.current_time;
        }
        if self.clock_range.is_none() {
            self.clock_range = source.clock_range;
        }
        if self.clock_step.is_none() {
            self.clock_step = source.clock_step;
        }
        if self.multiplier.is_none() {
            self.multiplier = source.multiplier;
        }
    }

    /// Gets the value as a Clock instance. Unset fields use defaults:
    /// clock_range=UNBOUNDED, clock_step=SYSTEM_CLOCK_MULTIPLIER, multiplier=1.0.
    ///
    /// Maps to `DataSourceClock.prototype.getValue`
    pub fn get_value(&self) -> Clock {
        let options = ClockOptions {
            start_time: self.start_time,
            stop_time: self.stop_time,
            current_time: self.current_time,
            multiplier: self.multiplier,
            clock_step: self.clock_step,
            clock_range: self.clock_range,
            can_animate: Some(true),
            should_animate: Some(true),
        };
        Clock::from_options(&options)
    }
}
