//! Ported from `packages/engine/Source/Core/TimeIntervalCollection.js`
//! (the subset required by the Composite property family).
//!
//! A non-overlapping collection of time intervals where each interval
//! carries a [`Property`] as its `data`, mirroring the CesiumJS
//! `TimeIntervalCollection` used by `CompositeProperty`.
//!
//! DEVIATION (structural): CesiumJS `TimeIntervalCollection` lives in Core
//! with `JulianDate` endpoints and arbitrary `data`; the Rust Core port
//! predates the `data` member, so this crate-local variant stores
//! `Box<dyn Property>` data with the crate-wide `f64` seconds time
//! convention while mirroring the exact `addInterval` merge/split,
//! `indexOf`, `findDataForIntervalContainingDate`, `equals` and
//! `changedEvent` semantics.

use std::rc::Rc;

use cesium_core::binary_search::binary_search;
use cesium_core::event::Event;

use crate::property::Property;

/// Comparator mirroring `compareIntervalStartTimes`.
fn compare_interval_start_times(left_start: f64, right_start: f64) -> f64 {
    left_start - right_start
}

/// A time interval with associated property data (mirrors `TimeInterval`
/// with its `data` member, using `f64` seconds instead of `JulianDate`).
pub struct CompositeInterval {
    /// The start time of the interval (seconds).
    pub start: f64,
    /// The stop time of the interval (seconds).
    pub stop: f64,
    /// Whether the start time is included in the interval.
    pub is_start_included: bool,
    /// Whether the stop time is included in the interval.
    pub is_stop_included: bool,
    /// The property associated with this interval (JS `interval.data`).
    ///
    /// Stored in an `Rc` to mirror the JS object-reference semantics: the
    /// same property instance may be shared by several intervals (e.g.
    /// after `addInterval` splits) and compared by identity.
    pub data: Rc<dyn Property>,
}

impl CompositeInterval {
    /// Creates a new interval (mirrors `new TimeInterval({start, stop,
    /// isStartIncluded, isStopIncluded, data})`).
    pub fn new(
        start: f64,
        stop: f64,
        is_start_included: bool,
        is_stop_included: bool,
        data: Rc<dyn Property>,
    ) -> Self {
        Self {
            start,
            stop,
            is_start_included,
            is_stop_included,
            data,
        }
    }

    /// Port of `TimeInterval.isEmpty`.
    pub fn is_empty(&self) -> bool {
        if self.stop < self.start {
            return true;
        }
        self.stop == self.start && !(self.is_start_included && self.is_stop_included)
    }

    /// Port of `TimeInterval.contains` (with `f64` seconds).
    pub fn contains(&self, time: f64) -> bool {
        if self.is_empty() {
            return false;
        }
        if time == self.start {
            return self.is_start_included;
        }
        if time == self.stop {
            return self.is_stop_included;
        }
        self.start < time && time < self.stop
    }

    /// Port of `TimeInterval.equals(left, right, dataComparer)`.
    pub fn equals(
        &self,
        other: &CompositeInterval,
        data_comparer: Option<&dyn Fn(&dyn Property, &dyn Property) -> bool>,
    ) -> bool {
        self.start == other.start
            && self.stop == other.stop
            && self.is_start_included == other.is_start_included
            && self.is_stop_included == other.is_stop_included
            && match data_comparer {
                Some(comparer) => comparer(self.data.as_ref(), other.data.as_ref()),
                None => Rc::ptr_eq(&self.data, &other.data),
            }
    }
}

/// A non-overlapping collection of [`CompositeInterval`] instances.
pub struct CompositeIntervalCollection {
    intervals: Vec<CompositeInterval>,
    changed_event: Event<()>,
}

impl CompositeIntervalCollection {
    /// Port of `new TimeIntervalCollection()`.
    pub fn new() -> Self {
        Self {
            intervals: Vec::new(),
            changed_event: Event::new(),
        }
    }

    /// Port of the `changedEvent` member.
    pub fn changed_event(&self) -> &Event<()> {
        &self.changed_event
    }

    /// Port of the `length` getter.
    pub fn length(&self) -> usize {
        self.intervals.len()
    }

    /// Port of the `isEmpty` getter.
    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    /// Port of the `start` getter (JS `undefined` -> `None`).
    pub fn start(&self) -> Option<f64> {
        self.intervals.first().map(|interval| interval.start)
    }

    /// Port of the `stop` getter (JS `undefined` -> `None`).
    pub fn stop(&self) -> Option<f64> {
        self.intervals.last().map(|interval| interval.stop)
    }

    /// Port of `get(index)`.
    pub fn get(&self, index: usize) -> Option<&CompositeInterval> {
        self.intervals.get(index)
    }

    /// Port of `removeAll()`: raises `changedEvent` only when the
    /// collection was non-empty.
    pub fn remove_all(&mut self) {
        if !self.intervals.is_empty() {
            self.intervals.clear();
            self.changed_event.raise_event(&());
        }
    }

    /// Port of `indexOf(date)`: returns the index of the interval
    /// containing `time`, or a negative value that is the bitwise
    /// complement of the insertion index (JS semantics).
    pub fn index_of(&self, time: f64) -> i64 {
        let intervals = &self.intervals;
        let search = binary_search(intervals, &time, |a: &CompositeInterval, b: &f64| {
            compare_interval_start_times(a.start, *b)
        });
        if search >= 0 {
            let index = search as usize;
            if intervals[index].is_start_included {
                return search;
            }

            if index > 0
                && intervals[index - 1].stop == time
                && intervals[index - 1].is_stop_included
            {
                return index as i64 - 1;
            }
            return !search;
        }

        let index = (!search) as usize;
        if index > 0 && index - 1 < intervals.len() && intervals[index - 1].contains(time) {
            return index as i64 - 1;
        }
        !search
    }

    /// Port of `findIntervalContainingDate(date)`: `indexOf` may return the
    /// interval preceding a gap time, so the candidate must be filtered by
    /// `TimeInterval.contains` (JS semantics).
    pub fn find_interval_containing_date(&self, time: f64) -> Option<&CompositeInterval> {
        let index = self.index_of(time);
        if index >= 0 {
            let interval = self.intervals.get(index as usize)?;
            if interval.contains(time) {
                return Some(interval);
            }
        }
        None
    }

    /// Port of `findDataForIntervalContainingDate(date)`.
    pub fn find_data_for_interval_containing_date(&self, time: f64) -> Option<&dyn Property> {
        self.find_interval_containing_date(time)
            .map(|interval| interval.data.as_ref())
    }

    /// Port of `contains(julianDate)`.
    pub fn contains(&self, time: f64) -> bool {
        self.find_interval_containing_date(time).is_some()
    }

    /// Port of `addInterval(interval, dataComparer)`: merges intervals with
    /// equal data and splits intervals with different data to maintain a
    /// non-overlapping collection; the new interval's data takes
    /// precedence. Raises `changedEvent` when the collection changes.
    pub fn add_interval(
        &mut self,
        mut interval: CompositeInterval,
        data_comparer: Option<&dyn Fn(&dyn Property, &dyn Property) -> bool>,
    ) {
        if interval.is_empty() {
            return;
        }

        let data_equal = |existing: &Rc<dyn Property>, new: &Rc<dyn Property>| match data_comparer {
            Some(comparer) => comparer(existing.as_ref(), new.as_ref()),
            None => Rc::ptr_eq(existing, new),
        };

        let intervals = &mut self.intervals;

        // Handle the common case quickly: we're adding a new interval which
        // is after all existing intervals.
        if intervals.is_empty() || interval.start > intervals[intervals.len() - 1].stop {
            intervals.push(interval);
            self.changed_event.raise_event(&());
            return;
        }

        // Keep the list sorted by the start date.
        let mut index = {
            let search = binary_search(intervals, &interval.start, |a: &CompositeInterval, b: &f64| {
                compare_interval_start_times(a.start, *b)
            });
            if search < 0 {
                (!search) as usize
            } else {
                let mut index = search as usize;
                // interval's start date exactly equals the start date of at
                // least one interval in the collection; disambiguate with
                // the surrounding `isStartIncluded` flags.
                if index > 0
                    && interval.is_start_included
                    && intervals[index - 1].is_start_included
                    && intervals[index - 1].start == interval.start
                {
                    index -= 1;
                } else if index < intervals.len()
                    && !interval.is_start_included
                    && intervals[index].is_start_included
                    && intervals[index].start == interval.start
                {
                    index += 1;
                }
                index
            }
        };

        if index > 0 {
            // Not the first thing in the list, so see if the interval before
            // this one overlaps this one.
            let previous_stop = intervals[index - 1].stop;
            let overlaps = previous_stop > interval.start
                || (previous_stop == interval.start
                    && (intervals[index - 1].is_stop_included || interval.is_start_included));
            if overlaps {
                if data_equal(&intervals[index - 1].data, &interval.data) {
                    // Overlapping intervals have the same data, so combine them.
                    let previous = intervals.remove(index - 1);
                    index -= 1;
                    if interval.stop > previous.stop {
                        interval = CompositeInterval::new(
                            previous.start,
                            interval.stop,
                            previous.is_start_included,
                            interval.is_stop_included,
                            interval.data,
                        );
                    } else {
                        interval = CompositeInterval::new(
                            previous.start,
                            previous.stop,
                            previous.is_start_included,
                            previous.is_stop_included
                                || (interval.stop == previous.stop && interval.is_stop_included),
                            interval.data,
                        );
                    }
                } else {
                    // Overlapping intervals have different data. The new
                    // interval being added "wins" so truncate the previous
                    // interval. If the existing interval extends past the
                    // end of the new one, split it into two intervals.
                    let previous_stop = intervals[index - 1].stop;
                    let previous_is_stop_included = intervals[index - 1].is_stop_included;
                    let extends_past = previous_stop > interval.stop
                        || (previous_stop == interval.stop
                            && previous_is_stop_included
                            && !interval.is_stop_included);
                    if extends_past {
                        let split = CompositeInterval::new(
                            interval.stop,
                            previous_stop,
                            !interval.is_stop_included,
                            previous_is_stop_included,
                            Rc::clone(&intervals[index - 1].data),
                        );
                        intervals.insert(index, split);
                    }
                    intervals[index - 1].stop = interval.start;
                    intervals[index - 1].is_stop_included = !interval.is_start_included;
                }
            }
        }

        while index < intervals.len() {
            // See if the intervals after this one overlap this one.
            let next_start = intervals[index].start;
            let overlaps = interval.stop > next_start
                || (interval.stop == next_start
                    && (interval.is_stop_included || intervals[index].is_start_included));
            if overlaps {
                if data_equal(&intervals[index].data, &interval.data) {
                    // Overlapping intervals have the same data, so combine them.
                    let next = intervals.remove(index);
                    let (stop, is_stop_included) = if next.stop > interval.stop {
                        (next.stop, next.is_stop_included)
                    } else {
                        (interval.stop, interval.is_stop_included)
                    };
                    interval = CompositeInterval::new(
                        interval.start,
                        stop,
                        interval.is_start_included,
                        is_stop_included,
                        interval.data,
                    );
                } else {
                    // Different data: the new interval wins; truncate the
                    // next interval.
                    intervals[index].start = interval.stop;
                    intervals[index].is_start_included = !interval.is_stop_included;

                    if intervals[index].is_empty() {
                        intervals.remove(index);
                    } else {
                        // Found a partial span, so it is not possible for
                        // the next interval to be spanned at all.
                        break;
                    }
                }
            } else {
                // Found the last one we're spanning, so stop looking.
                break;
            }
        }

        // Add the new interval.
        intervals.insert(index, interval);
        self.changed_event.raise_event(&());
    }

    /// Port of `removeInterval(interval)`: creates a hole over the span of
    /// `interval` (its `data` is ignored). Returns `true` when anything
    /// was removed and raises `changedEvent` in that case.
    pub fn remove_interval(&mut self, interval: &CompositeInterval) -> bool {
        if interval.is_empty() {
            return false;
        }

        let intervals = &mut self.intervals;
        let search = binary_search(intervals, &interval.start, |a: &CompositeInterval, b: &f64| {
            compare_interval_start_times(a.start, *b)
        });
        let mut index = if search < 0 { (!search) as usize } else { search as usize };

        let mut result = false;

        // Check for truncation of the end of the previous interval.
        if index > 0
            && (intervals[index - 1].stop > interval.start
                || (intervals[index - 1].stop == interval.start
                    && intervals[index - 1].is_stop_included
                    && interval.is_start_included))
        {
            result = true;

            if intervals[index - 1].stop > interval.stop
                || (intervals[index - 1].is_stop_included
                    && !interval.is_stop_included
                    && intervals[index - 1].stop == interval.stop)
            {
                // Break the existing interval into two pieces.
                let split = CompositeInterval::new(
                    interval.stop,
                    intervals[index - 1].stop,
                    !interval.is_stop_included,
                    intervals[index - 1].is_stop_included,
                    Rc::clone(&intervals[index - 1].data),
                );
                intervals.insert(index, split);
            }
            intervals[index - 1].stop = interval.start;
            intervals[index - 1].is_stop_included = !interval.is_start_included;
        }

        // Check if the start of the current interval should remain because
        // interval.start is the same but it is not included.
        if index < intervals.len()
            && !interval.is_start_included
            && intervals[index].is_start_included
            && interval.start == intervals[index].start
        {
            result = true;

            let point = CompositeInterval::new(
                intervals[index].start,
                intervals[index].start,
                true,
                true,
                Rc::clone(&intervals[index].data),
            );
            intervals.insert(index, point);
            index += 1;
        }

        // Remove any intervals that are completely overlapped by the input
        // interval.
        while index < intervals.len() && interval.stop > intervals[index].stop {
            result = true;
            intervals.remove(index);
        }

        // Check for the case where the input interval ends on the same date
        // as an existing interval.
        if index < intervals.len() && interval.stop == intervals[index].stop {
            result = true;

            if !interval.is_stop_included && intervals[index].is_stop_included {
                // Last point of interval should remain because the stop date
                // is included in the existing interval but is not included
                // in the input interval.
                if index + 1 < intervals.len()
                    && intervals[index + 1].start == interval.stop
                    && Rc::ptr_eq(&intervals[index].data, &intervals[index + 1].data)
                {
                    // Combine single point with the next interval.
                    intervals.remove(index);
                    intervals[index].is_start_included = true;
                } else {
                    let data = Rc::clone(&intervals[index].data);
                    intervals[index] =
                        CompositeInterval::new(interval.stop, interval.stop, true, true, data);
                }
            } else {
                // Interval is completely overlapped.
                intervals.remove(index);
            }
        }

        // Truncate any partially-overlapped intervals.
        if index < intervals.len()
            && (interval.stop > intervals[index].start
                || (interval.stop == intervals[index].start
                    && interval.is_stop_included
                    && intervals[index].is_start_included))
        {
            result = true;
            intervals[index].start = interval.stop;
            intervals[index].is_start_included = !interval.is_stop_included;
        }

        if result {
            self.changed_event.raise_event(&());
        }

        result
    }

    /// Port of `equals(right, dataComparer)`.
    pub fn equals(
        &self,
        other: &CompositeIntervalCollection,
        data_comparer: Option<&dyn Fn(&dyn Property, &dyn Property) -> bool>,
    ) -> bool {
        if self.intervals.len() != other.intervals.len() {
            return false;
        }
        for i in 0..self.intervals.len() {
            if !self.intervals[i].equals(&other.intervals[i], data_comparer) {
                return false;
            }
        }
        true
    }
}

impl Default for CompositeIntervalCollection {
    fn default() -> Self {
        Self::new()
    }
}
