//! Ported from `packages/engine/Source/DataSources/TimeIntervalCollectionProperty.js`.
//!
//! A [`Property`] which is defined by a time interval collection, where the
//! data of each interval represents the value at time.
//!
//! DEVIATION (structural): CesiumJS delegates to `Core/TimeIntervalCollection`
//! (with `JulianDate` times and full interval merging). The Rust port stores
//! the intervals locally with `f64` seconds (the crate-wide time convention)
//! and keeps them sorted by start time without the full merge/split
//! semantics of `TimeIntervalCollection.addInterval`; lookups implement the
//! exact `findDataForIntervalContainingDate` / `TimeInterval.contains`
//! semantics.
//!
//! DEVIATION (events): the `definitionChanged` event (raised by the JS
//! `intervals.changedEvent` subscription) is intentionally not implemented
//! here; the event system is owned by a separate work item.

use std::cmp::Ordering;

use cesium_core::binary_search::binary_search;

use crate::property::{Property, PropertyResult};

/// An interval with associated data (mirrors `Core/TimeInterval` with its
/// `data` member, using `f64` seconds instead of `JulianDate`).
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyTimeInterval {
    /// The start time of the interval (seconds).
    pub start: f64,
    /// The stop time of the interval (seconds).
    pub stop: f64,
    /// Whether the start time is included in the interval.
    pub is_start_included: bool,
    /// Whether the stop time is included in the interval.
    pub is_stop_included: bool,
    /// The data associated with this interval.
    pub data: PropertyResult,
}

impl PropertyTimeInterval {
    /// Creates a new interval with both endpoints included (mirrors the
    /// `TimeInterval` constructor defaults).
    pub fn new(start: f64, stop: f64, data: PropertyResult) -> Self {
        Self {
            start,
            stop,
            is_start_included: true,
            is_stop_included: true,
            data,
        }
    }

    /// Port of `TimeInterval.isEmpty()`.
    pub fn is_empty(&self) -> bool {
        match self.stop.partial_cmp(&self.start) {
            Some(Ordering::Less) => true,
            Some(Ordering::Equal) => !self.is_start_included || !self.is_stop_included,
            _ => false,
        }
    }

    /// Port of `TimeInterval.contains(julianDate)` (with `f64` seconds).
    pub fn contains(&self, time: f64) -> bool {
        if self.is_empty() {
            return false;
        }
        match self.start.partial_cmp(&time) {
            Some(Ordering::Equal) => return self.is_start_included,
            None => return false,
            _ => {}
        }
        match self.stop.partial_cmp(&time) {
            Some(Ordering::Equal) => return self.is_stop_included,
            None => return false,
            _ => {}
        }
        self.start < time && time < self.stop
    }
}

/// A [`Property`] which is defined by a collection of time intervals, where
/// the data of each interval represents the value at time.
pub struct TimeIntervalCollectionProperty {
    intervals: Vec<PropertyTimeInterval>,
}

impl TimeIntervalCollectionProperty {
    /// Port of `new TimeIntervalCollectionProperty()`.
    pub fn new() -> Self {
        Self {
            intervals: Vec::new(),
        }
    }

    /// Read access to the interval collection (JS `intervals` getter).
    pub fn intervals(&self) -> &[PropertyTimeInterval] {
        &self.intervals
    }

    /// Whether the interval collection is empty (JS `intervals.isEmpty`).
    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    /// Port of `intervals.addInterval(interval)`.
    ///
    /// DEVIATION: intervals are kept sorted by start time; the full merging
    /// behavior of `TimeIntervalCollection.addInterval` is not implemented.
    pub fn add_interval(&mut self, interval: PropertyTimeInterval) {
        let index = binary_search(&self.intervals, &interval.start, |a: &PropertyTimeInterval, b: &f64| {
            a.start - *b
        });
        let index = if index < 0 { (!index) as usize } else { index as usize + 1 };
        self.intervals.insert(index, interval);
    }

    /// Port of `intervals.removeInterval(interval)`. Removes the first
    /// interval with matching endpoints, inclusion flags, and data.
    /// Returns `true` if an interval was removed.
    pub fn remove_interval(&mut self, interval: &PropertyTimeInterval) -> bool {
        if let Some(index) = self.intervals.iter().position(|candidate| candidate == interval) {
            self.intervals.remove(index);
            true
        } else {
            false
        }
    }

    /// Port of `intervals.removeAll()`.
    pub fn remove_all(&mut self) {
        self.intervals.clear();
    }

    /// Port of `TimeIntervalCollection.findDataForIntervalContainingDate(julianDate)`.
    pub fn find_data_for_interval_containing_date(&self, time: f64) -> Option<PropertyResult> {
        let index = self.find_interval(time)?;
        Some(self.intervals[index].data.clone())
    }

    /// Mirrors `TimeIntervalCollection.indexOf(date)`: returns the index of
    /// the interval containing `time`, or `None` (the JS negative
    /// bitwise-complement result is not needed for property lookups).
    fn find_interval(&self, time: f64) -> Option<usize> {
        let intervals = &self.intervals;
        let search = binary_search(intervals, &time, |a: &PropertyTimeInterval, b: &f64| {
            a.start - *b
        });
        if search >= 0 {
            let index = search as usize;
            if intervals[index].is_start_included {
                return Some(index);
            }

            if index > 0
                && intervals[index - 1].stop == time
                && intervals[index - 1].is_stop_included
            {
                return Some(index - 1);
            }
            return None;
        }

        let index = (!search) as usize;
        if index > 0 && index - 1 < intervals.len() && intervals[index - 1].contains(time) {
            return Some(index - 1);
        }
        None
    }

    /// Port of `getValue(time)`. Returns `None` when no interval contains
    /// the time. The stored data is cloned, mirroring the JS
    /// `value.clone(result)` behavior for clonable values.
    pub fn get_value_option(&self, time: f64) -> Option<PropertyResult> {
        self.find_data_for_interval_containing_date(time)
    }

    /// Port of `equals(other)` for two [`TimeIntervalCollectionProperty`]
    /// instances (mirrors `intervals.equals(other, Property.equals)`).
    pub fn equals(&self, other: &TimeIntervalCollectionProperty) -> bool {
        self.intervals == other.intervals
    }
}

impl Default for TimeIntervalCollectionProperty {
    fn default() -> Self {
        Self::new()
    }
}

impl Property for TimeIntervalCollectionProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        self.get_value_option(time).unwrap_or(PropertyResult::None)
    }

    fn is_constant(&self) -> bool {
        self.intervals.is_empty()
    }

    fn is_destroyed(&self) -> bool {
        false
    }
}
