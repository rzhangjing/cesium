//! Ported from `packages/engine/Source/DataSources/DataSourceClock.js`.

/// Defines the clock settings for a data source.
pub struct DataSourceClock {
    /// The start time (Julian date as f64).
    pub start: f64,
    /// The stop time.
    pub stop: f64,
    /// The current time.
    pub current_time: f64,
    /// The multiplier (speed).
    pub multiplier: f64,
}

impl DataSourceClock {
    /// Creates a new data source clock.
    pub fn new() -> Self {
        Self {
            start: 0.0,
            stop: 0.0,
            current_time: 0.0,
            multiplier: 1.0,
        }
    }

    /// Merges another clock's settings into this one.
    pub fn merge(&mut self, other: &DataSourceClock) {
        if other.start != 0.0 { self.start = other.start; }
        if other.stop != 0.0 { self.stop = other.stop; }
        if other.multiplier != 0.0 { self.multiplier = other.multiplier; }
    }
}

impl Default for DataSourceClock {
    fn default() -> Self { Self::new() }
}
