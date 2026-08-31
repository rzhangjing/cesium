//! Ported from `packages/engine/Source/Core/TimeIntervalCollection.js`.
//!
//! A non-overlapping collection of [`TimeInterval`] instances sorted by start
//! time.
//!
//! DEVIATION (changed event argument): the JS `changedEvent` raises with the
//! collection itself as the argument; the Rust [`Event`] carries `()` because
//! listeners cannot receive a borrow of the collection while it is being
//! mutated.
//!
//! DEVIATION (data payload): interval data follows the [`IntervalData`]
//! restrictions of [`crate::time_interval`]; the JS default data comparer is
//! `===` (reference equality for objects), which the port mirrors through
//! [`IntervalData`] identity semantics.

use crate::binary_search::binary_search;
use crate::developer_error::throw_developer_error;
use crate::event::Event;
use crate::gregorian_date::GregorianDate;
use crate::is_leap_year::is_leap_year;
use crate::iso8601::Iso8601;
use crate::julian_date::JulianDate;
use crate::time_interval::{DataComparer, IntervalData, MergeCallback, TimeInterval};

/// Port of the module-level `compareIntervalStartTimes` comparator.
#[must_use]
pub fn compare_interval_start_times(left: &TimeInterval, right: &TimeInterval) -> i32 {
    JulianDate::compare(&left.start, &right.start)
}

/// Callback that returns the data for each interval created by the
/// `fromJulianDateArray` / `fromIso8601*` factory functions (port of the JS
/// `options.dataCallback`). Called with the interval and its index.
pub type DataCallback<'a> = dyn Fn(&TimeInterval, usize) -> Option<IntervalData> + 'a;

/// Options for [`TimeIntervalCollection::find_interval`] (port of the JS
/// `findInterval(options)` options object). All fields are optional; `None`
/// fields are treated as don't-care conditions.
#[derive(Default)]
pub struct FindIntervalOptions {
    /// The start time of the interval.
    pub start: Option<JulianDate>,
    /// The stop time of the interval.
    pub stop: Option<JulianDate>,
    /// Whether the start time is included in the interval.
    pub is_start_included: Option<bool>,
    /// Whether the stop time is included in the interval.
    pub is_stop_included: Option<bool>,
}

/// Options for [`TimeIntervalCollection::from_julian_date_array`].
#[derive(Default)]
pub struct FromJulianDateArrayOptions<'a> {
    /// An array of dates (JS `options.julianDates`).
    pub julian_dates: Vec<JulianDate>,
    /// Whether start time is included in the interval (default `true`).
    pub is_start_included: Option<bool>,
    /// Whether stop time is included in the interval (default `true`).
    pub is_stop_included: Option<bool>,
    /// Add an interval from `Iso8601::minimum_value()` to the start time.
    pub leading_interval: bool,
    /// Add an interval from the stop time to `Iso8601::maximum_value()`.
    pub trailing_interval: bool,
    /// Returns the data for each interval before it is added; when `None`,
    /// the data is the index in the collection.
    pub data_callback: Option<&'a DataCallback<'a>>,
}

/// Options for [`TimeIntervalCollection::from_iso8601`].
pub struct FromIso8601Options<'a> {
    /// An ISO 8601 interval (start/stop or start/stop/duration).
    pub iso8601: &'a str,
    /// Whether start time is included in the interval (default `true`).
    pub is_start_included: Option<bool>,
    /// Whether stop time is included in the interval (default `true`).
    pub is_stop_included: Option<bool>,
    /// Add an interval from `Iso8601::minimum_value()` to the start time.
    pub leading_interval: bool,
    /// Add an interval from the stop time to `Iso8601::maximum_value()`.
    pub trailing_interval: bool,
    /// Returns the data for each interval before it is added; when `None`,
    /// the data is the index in the collection.
    pub data_callback: Option<&'a DataCallback<'a>>,
}

/// Options for [`TimeIntervalCollection::from_iso8601_date_array`].
pub struct FromIso8601DateArrayOptions<'a> {
    /// An array of ISO 8601 dates.
    pub iso8601_dates: &'a [&'a str],
    /// Whether start time is included in the interval (default `true`).
    pub is_start_included: Option<bool>,
    /// Whether stop time is included in the interval (default `true`).
    pub is_stop_included: Option<bool>,
    /// Add an interval from `Iso8601::minimum_value()` to the start time.
    pub leading_interval: bool,
    /// Add an interval from the stop time to `Iso8601::maximum_value()`.
    pub trailing_interval: bool,
    /// Returns the data for each interval before it is added; when `None`,
    /// the data is the index in the collection.
    pub data_callback: Option<&'a DataCallback<'a>>,
}

/// Options for [`TimeIntervalCollection::from_iso8601_duration_array`].
pub struct FromIso8601DurationArrayOptions<'a> {
    /// A date that the durations are relative to.
    pub epoch: JulianDate,
    /// An array of ISO 8601 durations.
    pub iso8601_durations: &'a [&'a str],
    /// `true` if durations are relative to the previous date, `false` if
    /// always relative to the epoch.
    pub relative_to_previous: bool,
    /// Whether start time is included in the interval (default `true`).
    pub is_start_included: Option<bool>,
    /// Whether stop time is included in the interval (default `true`).
    pub is_stop_included: Option<bool>,
    /// Add an interval from `Iso8601::minimum_value()` to the start time.
    pub leading_interval: bool,
    /// Add an interval from the stop time to `Iso8601::maximum_value()`.
    pub trailing_interval: bool,
    /// Returns the data for each interval before it is added; when `None`,
    /// the data is the index in the collection.
    pub data_callback: Option<&'a DataCallback<'a>>,
}

/// A non-overlapping collection of [`TimeInterval`] instances sorted by start
/// time.
pub struct TimeIntervalCollection {
    intervals: Vec<TimeInterval>,
    changed_event: Event<()>,
}

impl TimeIntervalCollection {
    /// Creates a new empty collection (port of `new TimeIntervalCollection()`).
    #[must_use]
    pub fn new() -> Self {
        Self {
            intervals: Vec::new(),
            changed_event: Event::new(),
        }
    }

    /// Creates a collection from an array of intervals (port of `new
    /// TimeIntervalCollection(intervals)`); each interval is added with the
    /// full merge/split semantics of [`Self::add_interval`].
    #[must_use]
    pub fn from_intervals(intervals: Vec<TimeInterval>) -> Self {
        let mut result = Self::new();
        for interval in intervals {
            result.add_interval(interval);
        }
        result
    }

    /// Gets an event that is raised whenever the collection of intervals
    /// changes (port of the `changedEvent` getter).
    #[must_use]
    pub fn changed_event(&self) -> &Event<()> {
        &self.changed_event
    }

    /// Gets the start time of the collection (port of the `start` getter),
    /// or `None` when the collection is empty.
    #[must_use]
    pub fn start(&self) -> Option<JulianDate> {
        self.intervals.first().map(|i| i.start.clone())
    }

    /// Gets whether the start time is included in the collection (port of
    /// the `isStartIncluded` getter).
    #[must_use]
    pub fn is_start_included(&self) -> bool {
        self.intervals
            .first()
            .is_some_and(|i| i.is_start_included)
    }

    /// Gets the stop time of the collection (port of the `stop` getter), or
    /// `None` when the collection is empty.
    #[must_use]
    pub fn stop(&self) -> Option<JulianDate> {
        self.intervals.last().map(|i| i.stop.clone())
    }

    /// Gets whether the stop time is included in the collection (port of the
    /// `isStopIncluded` getter).
    #[must_use]
    pub fn is_stop_included(&self) -> bool {
        self.intervals.last().is_some_and(|i| i.is_stop_included)
    }

    /// Gets the number of intervals in the collection (port of the `length`
    /// getter).
    #[must_use]
    pub fn length(&self) -> usize {
        self.intervals.len()
    }

    /// Gets whether the collection is empty (port of the `isEmpty` getter).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    /// Compares this instance against the provided instance componentwise
    /// (port of `equals(right, dataComparer)`).
    #[must_use]
    pub fn equals(&self, right: &TimeIntervalCollection, data_comparer: Option<&DataComparer>) -> bool {
        if std::ptr::eq(self, right) {
            return true;
        }
        let intervals = &self.intervals;
        let right_intervals = &right.intervals;
        if intervals.len() != right_intervals.len() {
            return false;
        }
        for i in 0..intervals.len() {
            if !TimeInterval::equals_with(&intervals[i], &right_intervals[i], data_comparer) {
                return false;
            }
        }
        true
    }

    /// Gets the interval at the specified index (port of `get(index)`), or
    /// `None` if no interval exists at that index.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&TimeInterval> {
        self.intervals.get(index)
    }

    /// Removes all intervals from the collection (port of `removeAll()`).
    pub fn remove_all(&mut self) {
        if !self.intervals.is_empty() {
            self.intervals.clear();
            self.changed_event.raise_event(&());
        }
    }

    /// Finds and returns the interval that contains the specified date (port
    /// of `findIntervalContainingDate(date)`).
    ///
    /// The JS variant throws a `DeveloperError` for an undefined date; the
    /// Rust signature guarantees the date statically.
    #[must_use]
    pub fn find_interval_containing_date(&self, date: &JulianDate) -> Option<TimeInterval> {
        let index = self.index_of(date);
        if index >= 0 {
            self.intervals.get(index as usize).cloned()
        } else {
            None
        }
    }

    /// Finds and returns the data for the interval that contains the
    /// specified date (port of `findDataForIntervalContainingDate(date)`).
    ///
    /// The JS variant throws a `DeveloperError` for an undefined date; the
    /// Rust signature guarantees the date statically.
    #[must_use]
    pub fn find_data_for_interval_containing_date(
        &self,
        date: &JulianDate,
    ) -> Option<IntervalData> {
        let index = self.index_of(date);
        if index >= 0 {
            self.intervals.get(index as usize).and_then(|i| i.data.clone())
        } else {
            None
        }
    }

    /// Checks if the specified date is inside this collection (port of
    /// `contains(julianDate)`).
    ///
    /// The JS variant throws a `DeveloperError` for an undefined date; the
    /// Rust signature guarantees the date statically.
    #[must_use]
    pub fn contains(&self, julian_date: &JulianDate) -> bool {
        self.index_of(julian_date) >= 0
    }

    /// Finds and returns the index of the interval in the collection that
    /// contains the specified date (port of `indexOf(date)`). If no such
    /// interval exists, returns a negative number which is the bitwise
    /// complement of the index of the next interval that starts after the
    /// date, or the bitwise complement of the length of the collection.
    ///
    /// The JS variant throws a `DeveloperError` for an undefined date; the
    /// Rust signature guarantees the date statically.
    #[must_use]
    pub fn index_of(&self, date: &JulianDate) -> i64 {
        let intervals = &self.intervals;
        // indexOfScratch: a TimeInterval with start = stop = date.
        let scratch = TimeInterval::new(Some(date.clone()), Some(date.clone()), None, None);
        let index = binary_search(intervals, &scratch, |a: &TimeInterval, b: &TimeInterval| {
            compare_interval_start_times(a, b) as f64
        });
        if index >= 0 {
            let index = index as usize;
            if intervals[index].is_start_included {
                return index as i64;
            }

            if index > 0
                && JulianDate::equals(&intervals[index - 1].stop, date)
                && intervals[index - 1].is_stop_included
            {
                return index as i64 - 1;
            }
            return !(index as i64);
        }

        let index = (!index) as usize;
        if index > 0
            && index - 1 < intervals.len()
            && intervals[index - 1].contains(date)
        {
            return index as i64 - 1;
        }
        !(index as i64)
    }

    /// Returns the first interval in the collection that matches the
    /// specified parameters (port of `findInterval(options)`); `None` fields
    /// are treated as don't-care conditions.
    #[must_use]
    pub fn find_interval(&self, options: &FindIntervalOptions) -> Option<TimeInterval> {
        let start = &options.start;
        let stop = &options.stop;
        let is_start_included = options.is_start_included;
        let is_stop_included = options.is_stop_included;

        for interval in &self.intervals {
            if (start.is_none() || JulianDate::equals(&interval.start, start.as_ref().unwrap()))
                && (stop.is_none() || JulianDate::equals(&interval.stop, stop.as_ref().unwrap()))
                && (is_start_included.is_none()
                    || interval.is_start_included == is_start_included.unwrap())
                && (is_stop_included.is_none()
                    || interval.is_stop_included == is_stop_included.unwrap())
            {
                return Some(interval.clone());
            }
        }
        None
    }

    /// Adds an interval to the collection (port of `addInterval(interval)`
    /// without a data comparer; data is compared with `===`-equivalent
    /// [`IntervalData`] equality).
    pub fn add_interval(&mut self, interval: TimeInterval) {
        self.add_interval_with(interval, None);
    }

    /// Adds an interval to the collection, merging intervals that contain the
    /// same data and splitting intervals of different data as needed in order
    /// to maintain a non-overlapping collection. The data in the new interval
    /// takes precedence over any existing intervals in the collection (port
    /// of `addInterval(interval, dataComparer)`).
    pub fn add_interval_with(
        &mut self,
        mut interval: TimeInterval,
        data_comparer: Option<&DataComparer>,
    ) {
        // The JS variant throws a DeveloperError for an undefined interval;
        // the Rust signature guarantees the interval statically.
        if interval.is_empty() {
            return;
        }

        let same_data = |left: Option<&IntervalData>, right: Option<&IntervalData>| match data_comparer
        {
            Some(data_comparer) => data_comparer(left, right),
            None => left == right,
        };

        // Handle the common case quickly: we're adding a new interval which
        // is after all existing intervals.
        let fast_path = match self.intervals.last() {
            None => true,
            Some(last) => JulianDate::greater_than(&interval.start, &last.stop),
        };
        if fast_path {
            self.intervals.push(interval);
            self.changed_event.raise_event(&());
            return;
        }

        // Keep the list sorted by the start date
        let search = binary_search(&self.intervals, &interval, |a: &TimeInterval, b: &TimeInterval| {
            compare_interval_start_times(a, b) as f64
        });
        let mut index = if search < 0 {
            (!search) as usize
        } else {
            // interval's start date exactly equals the start date of at
            // least one interval in the collection. It could actually equal
            // the start date of two intervals if one of them does not
            // actually include the date. In that case, the binary search
            // could have found either. We need to look at the surrounding
            // intervals and their is_start_included properties in order to
            // make sure we're working with the correct interval.
            let mut index = search as usize;
            if index > 0
                && interval.is_start_included
                && self.intervals[index - 1].is_start_included
                && JulianDate::equals(&self.intervals[index - 1].start, &interval.start)
            {
                index -= 1;
            } else if index < self.intervals.len()
                && !interval.is_start_included
                && self.intervals[index].is_start_included
                && JulianDate::equals(&self.intervals[index].start, &interval.start)
            {
                index += 1;
            }
            index
        };

        if index > 0 {
            // Not the first thing in the list, so see if the interval before
            // this one overlaps this one.
            let comparison = JulianDate::compare(&self.intervals[index - 1].stop, &interval.start);
            if comparison > 0
                || (comparison == 0
                    && (self.intervals[index - 1].is_stop_included || interval.is_start_included))
            {
                // There is an overlap
                if same_data(
                    self.intervals[index - 1].data.as_ref(),
                    interval.data.as_ref(),
                ) {
                    // Overlapping intervals have the same data, so combine them
                    interval = if JulianDate::greater_than(&interval.stop, &self.intervals[index - 1].stop) {
                        TimeInterval::new_with_data(
                            Some(self.intervals[index - 1].start.clone()),
                            Some(interval.stop.clone()),
                            Some(self.intervals[index - 1].is_start_included),
                            Some(interval.is_stop_included),
                            interval.data.clone(),
                        )
                    } else {
                        let is_stop_included = self.intervals[index - 1].is_stop_included
                            || (JulianDate::equals(&interval.stop, &self.intervals[index - 1].stop)
                                && interval.is_stop_included);
                        TimeInterval::new_with_data(
                            Some(self.intervals[index - 1].start.clone()),
                            Some(self.intervals[index - 1].stop.clone()),
                            Some(self.intervals[index - 1].is_start_included),
                            Some(is_stop_included),
                            interval.data.clone(),
                        )
                    };
                    self.intervals.remove(index - 1);
                    index -= 1;
                } else {
                    // Overlapping intervals have different data. The new
                    // interval being added 'wins' so truncate the previous
                    // interval. If the existing interval extends past the end
                    // of the new one, split the existing interval into two
                    // intervals.
                    let comparison =
                        JulianDate::compare(&self.intervals[index - 1].stop, &interval.stop);
                    if comparison > 0
                        || (comparison == 0
                            && self.intervals[index - 1].is_stop_included
                            && !interval.is_stop_included)
                    {
                        let tail = TimeInterval::new_with_data(
                            Some(interval.stop.clone()),
                            Some(self.intervals[index - 1].stop.clone()),
                            Some(!interval.is_stop_included),
                            Some(self.intervals[index - 1].is_stop_included),
                            self.intervals[index - 1].data.clone(),
                        );
                        self.intervals.insert(index, tail);
                    }
                    let head = TimeInterval::new_with_data(
                        Some(self.intervals[index - 1].start.clone()),
                        Some(interval.start.clone()),
                        Some(self.intervals[index - 1].is_start_included),
                        Some(!interval.is_start_included),
                        self.intervals[index - 1].data.clone(),
                    );
                    self.intervals[index - 1] = head;
                }
            }
        }

        while index < self.intervals.len() {
            // Not the last thing in the list, so see if the intervals after
            // this one overlap this one.
            let comparison = JulianDate::compare(&interval.stop, &self.intervals[index].start);
            if comparison > 0
                || (comparison == 0
                    && (interval.is_stop_included || self.intervals[index].is_start_included))
            {
                // There is an overlap
                if same_data(self.intervals[index].data.as_ref(), interval.data.as_ref()) {
                    // Overlapping intervals have the same data, so combine them
                    let next_stop_greater =
                        JulianDate::greater_than(&self.intervals[index].stop, &interval.stop);
                    let start = interval.start.clone();
                    let is_start_included = interval.is_start_included;
                    let data = interval.data.clone();
                    let stop = if next_stop_greater {
                        self.intervals[index].stop.clone()
                    } else {
                        interval.stop.clone()
                    };
                    let is_stop_included = if next_stop_greater {
                        self.intervals[index].is_stop_included
                    } else {
                        interval.is_stop_included
                    };
                    interval = TimeInterval::new_with_data(
                        Some(start),
                        Some(stop),
                        Some(is_start_included),
                        Some(is_stop_included),
                        data,
                    );
                    self.intervals.remove(index);
                } else {
                    // Overlapping intervals have different data. The new
                    // interval being added 'wins' so truncate the next
                    // interval.
                    let truncated = TimeInterval::new_with_data(
                        Some(interval.stop.clone()),
                        Some(self.intervals[index].stop.clone()),
                        Some(!interval.is_stop_included),
                        Some(self.intervals[index].is_stop_included),
                        self.intervals[index].data.clone(),
                    );

                    if truncated.is_empty() {
                        self.intervals.remove(index);
                    } else {
                        self.intervals[index] = truncated;
                        // Found a partial span, so it is not possible for the
                        // next interval to be spanned at all. Stop looking.
                        break;
                    }
                }
            } else {
                // Found the last one we're spanning, so stop looking.
                break;
            }
        }

        // Add the new interval
        self.intervals.insert(index, interval);
        self.changed_event.raise_event(&());
    }

    /// Removes the specified interval from this interval collection, creating
    /// a hole over the specified interval. The data of the input interval is
    /// ignored (port of `removeInterval(interval)`). Returns `true` if any
    /// part of the interval was in the collection.
    pub fn remove_interval(&mut self, interval: &TimeInterval) -> bool {
        // The JS variant throws a DeveloperError for an undefined interval;
        // the Rust signature guarantees the interval statically.
        if interval.is_empty() {
            return false;
        }

        let mut index = binary_search(&self.intervals, interval, |a: &TimeInterval, b: &TimeInterval| {
            compare_interval_start_times(a, b) as f64
        });
        if index < 0 {
            index = !index;
        }
        let mut index = index as usize;

        let mut result = false;

        // Check for truncation of the end of the previous interval.
        if index > 0
            && (JulianDate::greater_than(&self.intervals[index - 1].stop, &interval.start)
                || (JulianDate::equals(&self.intervals[index - 1].stop, &interval.start)
                    && self.intervals[index - 1].is_stop_included
                    && interval.is_start_included))
        {
            result = true;

            if JulianDate::greater_than(&self.intervals[index - 1].stop, &interval.stop)
                || (self.intervals[index - 1].is_stop_included
                    && !interval.is_stop_included
                    && JulianDate::equals(&self.intervals[index - 1].stop, &interval.stop))
            {
                // Break the existing interval into two pieces
                let tail = TimeInterval::new_with_data(
                    Some(interval.stop.clone()),
                    Some(self.intervals[index - 1].stop.clone()),
                    Some(!interval.is_stop_included),
                    Some(self.intervals[index - 1].is_stop_included),
                    self.intervals[index - 1].data.clone(),
                );
                self.intervals.insert(index, tail);
            }
            let head = TimeInterval::new_with_data(
                Some(self.intervals[index - 1].start.clone()),
                Some(interval.start.clone()),
                Some(self.intervals[index - 1].is_start_included),
                Some(!interval.is_start_included),
                self.intervals[index - 1].data.clone(),
            );
            self.intervals[index - 1] = head;
        }

        // Check if the start of the current interval should remain because
        // interval.start is the same but it is not included.
        if index < self.intervals.len()
            && !interval.is_start_included
            && self.intervals[index].is_start_included
            && JulianDate::equals(&interval.start, &self.intervals[index].start)
        {
            result = true;

            let single_point = TimeInterval::new_with_data(
                Some(self.intervals[index].start.clone()),
                Some(self.intervals[index].start.clone()),
                Some(true),
                Some(true),
                self.intervals[index].data.clone(),
            );
            self.intervals.insert(index, single_point);
            index += 1;
        }

        // Remove any intervals that are completely overlapped by the input
        // interval.
        while index < self.intervals.len()
            && JulianDate::greater_than(&interval.stop, &self.intervals[index].stop)
        {
            result = true;
            self.intervals.remove(index);
        }

        // Check for the case where the input interval ends on the same date
        // as an existing interval.
        if index < self.intervals.len() && JulianDate::equals(&interval.stop, &self.intervals[index].stop)
        {
            result = true;

            if !interval.is_stop_included && self.intervals[index].is_stop_included {
                // Last point of interval should remain because the stop date
                // is included in the existing interval but is not included
                // in the input interval.
                if index + 1 < self.intervals.len()
                    && JulianDate::equals(&self.intervals[index + 1].start, &interval.stop)
                    && self.intervals[index].data == self.intervals[index + 1].data
                {
                    // Combine single point with the next interval
                    self.intervals.remove(index);
                    let combined = TimeInterval::new_with_data(
                        Some(self.intervals[index].start.clone()),
                        Some(self.intervals[index].stop.clone()),
                        Some(true),
                        Some(self.intervals[index].is_stop_included),
                        self.intervals[index].data.clone(),
                    );
                    self.intervals[index] = combined;
                } else {
                    let single_point = TimeInterval::new_with_data(
                        Some(interval.stop.clone()),
                        Some(interval.stop.clone()),
                        Some(true),
                        Some(true),
                        self.intervals[index].data.clone(),
                    );
                    self.intervals[index] = single_point;
                }
            } else {
                // Interval is completely overlapped
                self.intervals.remove(index);
            }
        }

        // Truncate any partially-overlapped intervals.
        if index < self.intervals.len()
            && (JulianDate::greater_than(&interval.stop, &self.intervals[index].start)
                || (JulianDate::equals(&interval.stop, &self.intervals[index].start)
                    && interval.is_stop_included
                    && self.intervals[index].is_start_included))
        {
            result = true;
            let truncated = TimeInterval::new_with_data(
                Some(interval.stop.clone()),
                Some(self.intervals[index].stop.clone()),
                Some(!interval.is_stop_included),
                Some(self.intervals[index].is_stop_included),
                self.intervals[index].data.clone(),
            );
            self.intervals[index] = truncated;
        }

        if result {
            self.changed_event.raise_event(&());
        }

        result
    }

    /// Creates a new instance that is the intersection of this collection and
    /// the provided collection (port of `intersect(other, dataComparer,
    /// mergeCallback)`).
    pub fn intersect(
        &self,
        other: &TimeIntervalCollection,
        data_comparer: Option<&DataComparer>,
        merge_callback: Option<&MergeCallback>,
    ) -> TimeIntervalCollection {
        // The JS variant throws a DeveloperError for an undefined `other`;
        // the Rust signature guarantees the collection statically.
        let result = TimeIntervalCollection::new();
        let mut result = result;
        let mut left = 0usize;
        let mut right = 0usize;
        let intervals = &self.intervals;
        let other_intervals = &other.intervals;

        while left < intervals.len() && right < other_intervals.len() {
            let left_interval = &intervals[left];
            let right_interval = &other_intervals[right];
            if JulianDate::less_than(&left_interval.stop, &right_interval.start) {
                left += 1;
            } else if JulianDate::less_than(&right_interval.stop, &left_interval.start) {
                right += 1;
            } else {
                // The following will add an intersection whose data is
                // 'merged' if the callback is defined
                if merge_callback.is_some()
                    || data_comparer.is_some_and(|data_comparer| {
                        data_comparer(left_interval.data.as_ref(), right_interval.data.as_ref())
                    })
                    || (data_comparer.is_none() && left_interval.data == right_interval.data)
                {
                    let intersection = TimeInterval::intersect_with_callback(
                        left_interval,
                        right_interval,
                        merge_callback,
                    );
                    if !intersection.is_empty() {
                        // Since we start with an empty collection for
                        // 'result', and there are no overlapping intervals in
                        // 'self' (as a rule), the 'intersection' will never
                        // overlap with a previous interval in 'result'. So,
                        // no need to do any additional 'merging'.
                        result.add_interval_with(intersection, data_comparer);
                    }
                }

                if JulianDate::less_than(&left_interval.stop, &right_interval.stop)
                    || (JulianDate::equals(&left_interval.stop, &right_interval.stop)
                        && !left_interval.is_stop_included
                        && right_interval.is_stop_included)
                {
                    left += 1;
                } else {
                    right += 1;
                }
            }
        }
        result
    }

    /// Creates a new instance from a JulianDate array (port of
    /// `fromJulianDateArray(options, result)`).
    pub fn from_julian_date_array(
        options: FromJulianDateArrayOptions,
        result: Option<TimeIntervalCollection>,
    ) -> TimeIntervalCollection {
        let mut result = result.unwrap_or_else(TimeIntervalCollection::new);

        let julian_dates = &options.julian_dates;
        let length = julian_dates.len();
        let data_callback = &options.data_callback;

        let is_start_included = options.is_start_included.unwrap_or(true);
        let is_stop_included = options.is_stop_included.unwrap_or(true);
        let leading_interval = options.leading_interval;
        let trailing_interval = options.trailing_interval;

        // Add a default interval, which will only end up being used up to
        // first interval
        let mut start_index = 0;
        if leading_interval {
            start_index += 1;
            let mut interval = TimeInterval::new(
                Some(Iso8601::minimum_value().clone()),
                Some(julian_dates[0].clone()),
                Some(true),
                Some(!is_start_included),
            );
            interval.data = match data_callback {
                Some(data_callback) => data_callback(&interval, result.length()),
                None => Some(IntervalData::Number(result.length() as f64)),
            };
            result.add_interval(interval);
        }

        for i in 0..length.saturating_sub(1) {
            let start_date = &julian_dates[i];
            let end_date = &julian_dates[i + 1];

            let mut interval = TimeInterval::new(
                Some(start_date.clone()),
                Some(end_date.clone()),
                Some(if result.length() == start_index {
                    is_start_included
                } else {
                    true
                }),
                Some(if i == length - 2 { is_stop_included } else { false }),
            );
            interval.data = match data_callback {
                Some(data_callback) => data_callback(&interval, result.length()),
                None => Some(IntervalData::Number(result.length() as f64)),
            };
            result.add_interval(interval);
        }

        if trailing_interval {
            let mut interval = TimeInterval::new(
                Some(julian_dates[length - 1].clone()),
                Some(Iso8601::maximum_value().clone()),
                Some(!is_stop_included),
                Some(true),
            );
            interval.data = match data_callback {
                Some(data_callback) => data_callback(&interval, result.length()),
                None => Some(IntervalData::Number(result.length() as f64)),
            };
            result.add_interval(interval);
        }

        result
    }

    /// Creates a new instance from an ISO 8601 time interval
    /// (start/end/duration) (port of `fromIso8601(options, result)`).
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when the interval string does not
    /// contain valid ISO 8601 dates.
    pub fn from_iso8601(
        options: FromIso8601Options,
        result: Option<TimeIntervalCollection>,
    ) -> TimeIntervalCollection {
        const INVALID_INTERVAL: &str = "options.iso8601 is an invalid ISO 8601 interval.";

        let dates: Vec<&str> = options.iso8601.split('/').collect();
        let start = dates
            .first()
            .and_then(|d| JulianDate::from_iso8601(d))
            .unwrap_or_else(|| throw_developer_error(INVALID_INTERVAL));
        let stop = dates
            .get(1)
            .and_then(|d| JulianDate::from_iso8601(d))
            .unwrap_or_else(|| throw_developer_error(INVALID_INTERVAL));
        let mut julian_dates = Vec::new();

        let duration = dates.get(2).and_then(|d| parse_duration(d));
        if duration.is_none() {
            julian_dates.push(start);
            julian_dates.push(stop);
        } else {
            let duration = duration.unwrap();
            let mut date = start.clone();
            julian_dates.push(date.clone());
            while JulianDate::compare(&date, &stop) < 0 {
                date = add_to_date(&date, &duration);
                let after_stop = JulianDate::compare(&stop, &date) <= 0;
                if after_stop {
                    date = stop.clone();
                }
                julian_dates.push(date.clone());
            }
        }

        Self::from_julian_date_array(
            FromJulianDateArrayOptions {
                julian_dates,
                is_start_included: options.is_start_included,
                is_stop_included: options.is_stop_included,
                leading_interval: options.leading_interval,
                trailing_interval: options.trailing_interval,
                data_callback: options.data_callback,
            },
            result,
        )
    }

    /// Creates a new instance from an ISO 8601 date array (port of
    /// `fromIso8601DateArray(options, result)`).
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when a date string is invalid.
    pub fn from_iso8601_date_array(
        options: FromIso8601DateArrayOptions,
        result: Option<TimeIntervalCollection>,
    ) -> TimeIntervalCollection {
        let julian_dates: Vec<JulianDate> = options
            .iso8601_dates
            .iter()
            .map(|date| {
                JulianDate::from_iso8601(date).unwrap_or_else(|| {
                    throw_developer_error("options.iso8601Dates contains an invalid ISO 8601 date.")
                })
            })
            .collect();

        Self::from_julian_date_array(
            FromJulianDateArrayOptions {
                julian_dates,
                is_start_included: options.is_start_included,
                is_stop_included: options.is_stop_included,
                leading_interval: options.leading_interval,
                trailing_interval: options.trailing_interval,
                data_callback: options.data_callback,
            },
            result,
        )
    }

    /// Creates a new instance from an ISO 8601 duration array (port of
    /// `fromIso8601DurationArray(options, result)`).
    pub fn from_iso8601_duration_array(
        options: FromIso8601DurationArrayOptions,
        result: Option<TimeIntervalCollection>,
    ) -> TimeIntervalCollection {
        let epoch = &options.epoch;
        let iso8601_durations = options.iso8601_durations;
        let relative_to_previous = options.relative_to_previous;
        let mut julian_dates = Vec::new();
        let mut previous_date: Option<JulianDate> = None;

        for (i, duration) in iso8601_durations.iter().enumerate() {
            let parsed = parse_duration(duration);
            // Allow a duration of 0 on the first iteration, because then it
            // is just the epoch
            if parsed.is_some() || i == 0 {
                let duration = parsed.unwrap_or_default();
                let date = if relative_to_previous {
                    match &previous_date {
                        Some(previous_date) => add_to_date(previous_date, &duration),
                        None => add_to_date(epoch, &duration),
                    }
                } else {
                    add_to_date(epoch, &duration)
                };
                julian_dates.push(date.clone());
                previous_date = Some(date);
            }
        }

        Self::from_julian_date_array(
            FromJulianDateArrayOptions {
                julian_dates,
                is_start_included: options.is_start_included,
                is_stop_included: options.is_stop_included,
                leading_interval: options.leading_interval,
                trailing_interval: options.trailing_interval,
                data_callback: options.data_callback,
            },
            result,
        )
    }
}

impl Default for TimeIntervalCollection {
    fn default() -> Self {
        Self::new()
    }
}

// ── private helpers (ported module-level functions) ────────────────────────

/// A duration with possibly fractional components (the JS `scratchDuration`
/// is a `GregorianDate` reused with floating-point values; the Rust port
/// keeps the fractional values until [`add_to_date`]).
#[derive(Default, Clone, Debug)]
struct DurationParts {
    year: f64,
    month: f64,
    day: f64,
    hour: f64,
    minute: f64,
    second: f64,
    millisecond: f64,
}

/// Port of `addToDate(julianDate, duration, result)`: adds a duration
/// represented as a Gregorian date to a Julian date, carrying each component
/// with Gregorian calendar rules.
fn add_to_date(julian_date: &JulianDate, duration: &DurationParts) -> JulianDate {
    let gregorian = julian_date.to_gregorian_date();

    let mut millisecond = gregorian.millisecond + duration.millisecond;
    let mut second = gregorian.second as f64 + duration.second;
    let mut minute = gregorian.minute as f64 + duration.minute;
    let mut hour = gregorian.hour as f64 + duration.hour;
    let mut day = gregorian.day as f64 + duration.day;
    let mut month = gregorian.month as f64 + duration.month;
    let mut year = gregorian.year as f64 + duration.year;

    if millisecond >= 1000.0 {
        second += (millisecond / 1000.0).floor();
        millisecond %= 1000.0;
    }

    if second >= 60.0 {
        minute += (second / 60.0).floor();
        second %= 60.0;
    }

    if minute >= 60.0 {
        hour += (minute / 60.0).floor();
        minute %= 60.0;
    }

    if hour >= 24.0 {
        day += (hour / 24.0).floor();
        hour %= 24.0;
    }

    // If days is greater than the month's length we need to remove those
    // number of days, readjust month and year and repeat until days is less
    // than the month's length.
    let mut month_lengths = [0.0, 31.0, 28.0, 31.0, 30.0, 31.0, 30.0, 31.0, 31.0, 30.0, 31.0, 30.0, 31.0];
    month_lengths[2] = if is_leap_year(year) { 29.0 } else { 28.0 };
    // monthLengths[month] is undefined in JS when month >= 13 (the loop body
    // only compares day against it while month is in range); mirror that.
    while month < 13.0 && day > month_lengths[month as usize] {
        day -= month_lengths[month as usize];
        month += 1.0;
        if month >= 13.0 {
            month -= 1.0;
            year += (month / 12.0).floor();
            month %= 12.0;
            month += 1.0;
        }
        month_lengths[2] = if is_leap_year(year) { 29.0 } else { 28.0 };
    }

    let result = GregorianDate::new(
        year as i32,
        month as i32,
        day as i32,
        hour as i32,
        minute as i32,
        second as i32,
        millisecond,
        gregorian.is_leap_second,
    );
    JulianDate::from_gregorian_date(&result)
}

/// Parses a single ISO 8601 duration component, applying the JS
/// `Number(matches[i].replace(",", "."))` conversion.
fn parse_duration_number(value: &str) -> f64 {
    value.replacen(',', ".", 1).parse::<f64>().unwrap_or(f64::NAN)
}

/// Port of `parseDuration(iso8601, result)`: parses an ISO 8601 duration
/// string. Returns `None` when parsing fails or the duration is zero (the JS
/// function returns `false` in both cases).
fn parse_duration(iso8601: &str) -> Option<DurationParts> {
    if iso8601.is_empty() {
        return None;
    }

    let mut result = DurationParts::default();

    if iso8601.starts_with('P') {
        let duration_regex = regex::Regex::new(
            r"P(?:([\d.,]+)Y)?(?:([\d.,]+)M)?(?:([\d.,]+)W)?(?:([\d.,]+)D)?(?:T(?:([\d.,]+)H)?(?:([\d.,]+)M)?(?:([\d.,]+)S)?)?",
        )
        .unwrap();
        let matches = duration_regex.captures(iso8601)?;
        let captures = matches;
        if let Some(years) = captures.get(1) {
            // Years
            result.year = parse_duration_number(years.as_str());
        }
        if let Some(months) = captures.get(2) {
            // Months
            result.month = parse_duration_number(months.as_str());
        }
        if let Some(weeks) = captures.get(3) {
            // Weeks
            result.day = parse_duration_number(weeks.as_str()) * 7.0;
        }
        if let Some(days) = captures.get(4) {
            // Days
            result.day += parse_duration_number(days.as_str());
        }
        if let Some(hours) = captures.get(5) {
            // Hours
            result.hour = parse_duration_number(hours.as_str());
        }
        if let Some(minutes) = captures.get(6) {
            // Minutes
            result.minute = parse_duration_number(minutes.as_str());
        }
        if let Some(seconds) = captures.get(7) {
            // Seconds
            let seconds = parse_duration_number(seconds.as_str());
            result.second = seconds.floor();
            result.millisecond = (seconds % 1.0) * 1000.0;
        }
    } else {
        // They can technically specify the duration as a normal date with
        // some caveats. Try our best to load it.
        let with_zone = if iso8601.ends_with('Z') {
            iso8601.to_owned()
        } else {
            // It's not a date, its a duration, so it always has to be UTC
            format!("{iso8601}Z")
        };
        let julian = JulianDate::from_iso8601(&with_zone)
            .unwrap_or_else(|| throw_developer_error("iso8601 is an invalid ISO 8601 duration."));
        let gregorian = julian.to_gregorian_date();
        result.year = gregorian.year as f64;
        result.month = gregorian.month as f64;
        result.day = gregorian.day as f64;
        result.hour = gregorian.hour as f64;
        result.minute = gregorian.minute as f64;
        result.second = gregorian.second as f64;
        result.millisecond = gregorian.millisecond;
    }

    // A duration of 0 will cause an infinite loop, so just make sure
    // something is non-zero
    if result.year != 0.0
        || result.month != 0.0
        || result.day != 0.0
        || result.hour != 0.0
        || result.minute != 0.0
        || result.second != 0.0
        || result.millisecond != 0.0
    {
        Some(result)
    } else {
        None
    }
}
