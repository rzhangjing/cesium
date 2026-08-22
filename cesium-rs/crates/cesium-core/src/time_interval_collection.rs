//! Ported from `packages/engine/Source/Core/TimeIntervalCollection.js`.
//!
//! A collection of time intervals.

use crate::time_interval::TimeInterval;

/// A non-overlapping collection of `TimeInterval` instances.
pub struct TimeIntervalCollection {
    intervals: Vec<TimeInterval>,
}

impl TimeIntervalCollection {
    /// Creates a new empty collection.
    pub fn new() -> Self {
        Self {
            intervals: Vec::new(),
        }
    }

    /// Returns the number of intervals.
    pub fn length(&self) -> usize {
        self.intervals.len()
    }

    /// Returns the interval at the given index.
    pub fn get(&self, index: usize) -> Option<&TimeInterval> {
        self.intervals.get(index)
    }

    /// Adds an interval to the collection.
    pub fn add_interval(&mut self, interval: TimeInterval) {
        self.intervals.push(interval);
    }

    /// Removes all intervals from the collection.
    pub fn remove_all(&mut self) {
        self.intervals.clear();
    }

    /// Whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }
}

impl Default for TimeIntervalCollection {
    fn default() -> Self {
        Self::new()
    }
}
