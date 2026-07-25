//! TimeIntervalCollection - a non-overlapping collection of `TimeInterval`
//! instances sorted by start time.
//!
//! Maps to CesiumJS `Core/TimeIntervalCollection.js`.
//!
//! The collection keeps intervals sorted by start time and guarantees that no
//! two intervals overlap. Adding an interval merges it with adjacent intervals
//! that carry the same data, or splits/truncates existing intervals when the
//! data differs (the newly added interval's data takes precedence).

use crate::julian_date::JulianDate;
use crate::time_interval::TimeInterval;
use std::cmp::Ordering;

/// A `TimeInterval` together with an optional data payload.
///
/// Maps to CesiumJS `TimeInterval` (which carries a `data` property).
#[derive(Debug, Clone, PartialEq)]
pub struct TimeIntervalData<T> {
    /// The underlying interval (start/stop/inclusion flags).
    pub interval: TimeInterval,
    /// The data associated with this interval.
    pub data: Option<T>,
}

impl<T> TimeIntervalData<T> {
    /// Creates a new interval with data.
    pub fn new(interval: TimeInterval, data: Option<T>) -> Self {
        Self { interval, data }
    }

    /// Returns true if the interval is empty.
    pub fn is_empty(&self) -> bool {
        self.interval.is_empty()
    }

    /// Returns true if the interval contains the given time.
    pub fn contains(&self, time: &JulianDate) -> bool {
        self.interval.contains(time)
    }
}

/// A non-overlapping collection of `TimeInterval` instances sorted by start time.
///
/// Maps to CesiumJS `TimeIntervalCollection`.
#[derive(Debug, Clone)]
pub struct TimeIntervalCollection<T> {
    intervals: Vec<TimeIntervalData<T>>,
}

impl<T> Default for TimeIntervalCollection<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TimeIntervalCollection<T> {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self {
            intervals: Vec::new(),
        }
    }

    /// Creates a collection pre-populated with the given intervals.
    pub fn from_intervals<F>(intervals: Vec<TimeIntervalData<T>>, same_data: &F) -> Self
    where
        T: Clone,
        F: Fn(&T, &T) -> bool,
    {
        let mut collection = Self::new();
        for interval in intervals {
            collection.add_interval(interval, same_data);
        }
        collection
    }

    /// The number of intervals in the collection.
    /// Maps to `TimeIntervalCollection.prototype.length`.
    pub fn len(&self) -> usize {
        self.intervals.len()
    }

    /// Returns true if the collection is empty.
    /// Maps to `TimeIntervalCollection.prototype.isEmpty`.
    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    /// The start time of the collection (start of the first interval).
    /// Maps to `TimeIntervalCollection.prototype.start`.
    pub fn start(&self) -> Option<JulianDate> {
        self.intervals.first().map(|i| i.interval.start)
    }

    /// Whether the start time is included in the collection.
    /// Maps to `TimeIntervalCollection.prototype.isStartIncluded`.
    pub fn is_start_included(&self) -> bool {
        self.intervals
            .first()
            .map(|i| i.interval.is_start_included)
            .unwrap_or(false)
    }

    /// The stop time of the collection (stop of the last interval).
    /// Maps to `TimeIntervalCollection.prototype.stop`.
    pub fn stop(&self) -> Option<JulianDate> {
        self.intervals.last().map(|i| i.interval.stop)
    }

    /// Whether the stop time is included in the collection.
    /// Maps to `TimeIntervalCollection.prototype.isStopIncluded`.
    pub fn is_stop_included(&self) -> bool {
        self.intervals
            .last()
            .map(|i| i.interval.is_stop_included)
            .unwrap_or(false)
    }

    /// Gets the interval at the specified index.
    /// Maps to `TimeIntervalCollection.prototype.get`.
    pub fn get(&self, index: usize) -> Option<&TimeIntervalData<T>> {
        self.intervals.get(index)
    }

    /// Returns an iterator over the intervals.
    pub fn iter(&self) -> std::slice::Iter<'_, TimeIntervalData<T>> {
        self.intervals.iter()
    }

    /// Removes all intervals from the collection.
    /// Maps to `TimeIntervalCollection.prototype.removeAll`.
    pub fn remove_all(&mut self) {
        self.intervals.clear();
    }

    /// Finds and returns the index of the interval that contains the specified
    /// date. Returns a negative number (bitwise complement of the insertion
    /// index) when no interval contains the date, matching CesiumJS semantics.
    ///
    /// Maps to `TimeIntervalCollection.prototype.indexOf`.
    pub fn index_of(&self, date: &JulianDate) -> isize {
        let intervals = &self.intervals;

        // Binary search on start times for an interval whose start == date.
        let mut index = binary_search_start(intervals, date);

        if index >= 0 {
            let idx = index as usize;
            if intervals[idx].interval.is_start_included {
                return index;
            }
            if idx > 0
                && intervals[idx - 1].interval.stop == *date
                && intervals[idx - 1].interval.is_stop_included
            {
                return (idx - 1) as isize;
            }
            return !(index);
        }

        index = !index;
        let idx = index as usize;
        if idx > 0
            && idx - 1 < intervals.len()
            && intervals[idx - 1].contains(date)
        {
            return (idx - 1) as isize;
        }
        !index
    }

    /// Returns true if the collection contains the specified date.
    /// Maps to `TimeIntervalCollection.prototype.contains`.
    pub fn contains(&self, date: &JulianDate) -> bool {
        self.index_of(date) >= 0
    }

    /// Finds and returns the interval that contains the specified date.
    /// Maps to `TimeIntervalCollection.prototype.findIntervalContainingDate`.
    pub fn find_interval_containing_date(&self, date: &JulianDate) -> Option<&TimeIntervalData<T>> {
        let index = self.index_of(date);
        if index >= 0 {
            self.intervals.get(index as usize)
        } else {
            None
        }
    }

    /// Finds and returns the data for the interval that contains the specified date.
    /// Maps to `TimeIntervalCollection.prototype.findDataForIntervalContainingDate`.
    pub fn find_data_for_interval_containing_date(&self, date: &JulianDate) -> Option<&T> {
        self.find_interval_containing_date(date)
            .and_then(|i| i.data.as_ref())
    }

    /// Returns the first interval matching the optional start/stop/inclusion
    /// parameters. `None` parameters are treated as don't-care.
    ///
    /// Maps to `TimeIntervalCollection.prototype.findInterval`.
    pub fn find_interval(
        &self,
        start: Option<&JulianDate>,
        stop: Option<&JulianDate>,
        is_start_included: Option<bool>,
        is_stop_included: Option<bool>,
    ) -> Option<&TimeIntervalData<T>> {
        self.intervals.iter().find(|interval| {
            let iv = &interval.interval;
            let start_ok = start.map(|s| iv.start == *s).unwrap_or(true);
            let stop_ok = stop.map(|s| iv.stop == *s).unwrap_or(true);
            let isi_ok = is_start_included
                .map(|v| iv.is_start_included == v)
                .unwrap_or(true);
            let ist_ok = is_stop_included
                .map(|v| iv.is_stop_included == v)
                .unwrap_or(true);
            start_ok && stop_ok && isi_ok && ist_ok
        })
    }

    /// Adds an interval to the collection, merging intervals that contain the
    /// same data and splitting intervals of different data as needed in order
    /// to maintain a non-overlapping collection. The data in the new interval
    /// takes precedence over any existing intervals.
    ///
    /// `same_data` compares two data payloads to decide whether adjacent
    /// intervals can be merged.
    ///
    /// Maps to `TimeIntervalCollection.prototype.addInterval`.
    pub fn add_interval<F>(&mut self, mut interval: TimeIntervalData<T>, same_data: &F)
    where
        T: Clone,
        F: Fn(&T, &T) -> bool,
    {
        if interval.is_empty() {
            return;
        }

        // Fast path: appending after everything already present.
        if self.intervals.is_empty()
            || interval.interval.start > self.intervals[self.intervals.len() - 1].interval.stop
        {
            self.intervals.push(interval);
            return;
        }

        // Keep the list sorted by start date.
        let mut index = binary_search_start(&self.intervals, &interval.interval.start);
        if index < 0 {
            index = !index;
        } else {
            let mut idx = index as usize;
            if idx > 0
                && interval.interval.is_start_included
                && self.intervals[idx - 1].interval.is_start_included
                && self.intervals[idx - 1].interval.start == interval.interval.start
            {
                idx -= 1;
            } else if idx < self.intervals.len()
                && !interval.interval.is_start_included
                && self.intervals[idx].interval.is_start_included
                && self.intervals[idx].interval.start == interval.interval.start
            {
                idx += 1;
            }
            index = idx as isize;
        }

        let mut idx = index as usize;

        if idx > 0 {
            // See if the interval before this one overlaps this one.
            let cmp = compare(
                &self.intervals[idx - 1].interval.stop,
                &interval.interval.start,
            );
            if cmp == Ordering::Greater
                || (cmp == Ordering::Equal
                    && (self.intervals[idx - 1].interval.is_stop_included
                        || interval.interval.is_start_included))
            {
                let same = data_equals(
                    self.intervals[idx - 1].data.as_ref(),
                    interval.data.as_ref(),
                    same_data,
                );
                if same {
                    // Overlapping intervals have the same data, so combine them.
                    if interval.interval.stop > self.intervals[idx - 1].interval.stop {
                        interval = TimeIntervalData {
                            interval: TimeInterval::new(
                                self.intervals[idx - 1].interval.start,
                                interval.interval.stop,
                                self.intervals[idx - 1].interval.is_start_included,
                                interval.interval.is_stop_included,
                            ),
                            data: interval.data,
                        };
                    } else {
                        let stop_included = self.intervals[idx - 1].interval.is_stop_included
                            || (interval.interval.stop == self.intervals[idx - 1].interval.stop
                                && interval.interval.is_stop_included);
                        interval = TimeIntervalData {
                            interval: TimeInterval::new(
                                self.intervals[idx - 1].interval.start,
                                self.intervals[idx - 1].interval.stop,
                                self.intervals[idx - 1].interval.is_start_included,
                                stop_included,
                            ),
                            data: interval.data,
                        };
                    }
                    self.intervals.remove(idx - 1);
                    idx -= 1;
                } else {
                    // Different data: the new interval wins; truncate the previous
                    // interval, splitting it if it extends past the new one.
                    let cmp2 = compare(
                        &self.intervals[idx - 1].interval.stop,
                        &interval.interval.stop,
                    );
                    if cmp2 == Ordering::Greater
                        || (cmp2 == Ordering::Equal
                            && self.intervals[idx - 1].interval.is_stop_included
                            && !interval.interval.is_stop_included)
                    {
                        let tail = TimeIntervalData {
                            interval: TimeInterval::new(
                                interval.interval.stop,
                                self.intervals[idx - 1].interval.stop,
                                !interval.interval.is_stop_included,
                                self.intervals[idx - 1].interval.is_stop_included,
                            ),
                            data: self.intervals[idx - 1].data.clone(),
                        };
                        self.intervals.insert(idx, tail);
                    }
                    let prev = &self.intervals[idx - 1];
                    let truncated = TimeIntervalData {
                        interval: TimeInterval::new(
                            prev.interval.start,
                            interval.interval.start,
                            prev.interval.is_start_included,
                            !interval.interval.is_start_included,
                        ),
                        data: prev.data.clone(),
                    };
                    self.intervals[idx - 1] = truncated;
                }
            }
        }

        while idx < self.intervals.len() {
            // See if the intervals after this one overlap this one.
            let cmp = compare(&interval.interval.stop, &self.intervals[idx].interval.start);
            if cmp == Ordering::Greater
                || (cmp == Ordering::Equal
                    && (interval.interval.is_stop_included
                        || self.intervals[idx].interval.is_start_included))
            {
                let same = data_equals(
                    self.intervals[idx].data.as_ref(),
                    interval.data.as_ref(),
                    same_data,
                );
                if same {
                    // Same data: combine them.
                    let next_stop = self.intervals[idx].interval.stop;
                    let (new_stop, new_stop_included) = if next_stop > interval.interval.stop {
                        (next_stop, self.intervals[idx].interval.is_stop_included)
                    } else {
                        (interval.interval.stop, interval.interval.is_stop_included)
                    };
                    interval = TimeIntervalData {
                        interval: TimeInterval::new(
                            interval.interval.start,
                            new_stop,
                            interval.interval.is_start_included,
                            new_stop_included,
                        ),
                        data: interval.data,
                    };
                    self.intervals.remove(idx);
                } else {
                    // Different data: the new interval wins; truncate the next interval.
                    let next = &self.intervals[idx];
                    let truncated = TimeIntervalData {
                        interval: TimeInterval::new(
                            interval.interval.stop,
                            next.interval.stop,
                            !interval.interval.is_stop_included,
                            next.interval.is_stop_included,
                        ),
                        data: next.data.clone(),
                    };
                    if truncated.is_empty() {
                        self.intervals.remove(idx);
                    } else {
                        self.intervals[idx] = truncated;
                        // Found a partial span; the next interval cannot be spanned.
                        break;
                    }
                }
            } else {
                // Found the last one we're spanning; stop looking.
                break;
            }
        }

        self.intervals.insert(idx, interval);
    }

    /// Removes the specified interval from this collection, creating a hole
    /// over the specified interval. The data of the input interval is ignored.
    /// Returns true if any part of the interval was in the collection.
    ///
    /// Maps to `TimeIntervalCollection.prototype.removeInterval`.
    pub fn remove_interval(&mut self, interval: &TimeInterval) -> bool
    where
        T: Clone,
    {
        if interval.is_empty() {
            return false;
        }

        let mut index = binary_search_start(&self.intervals, &interval.start);
        if index < 0 {
            index = !index;
        }
        let mut idx = index as usize;

        let mut result = false;

        // Check for truncation of the end of the previous interval.
        if idx > 0
            && (self.intervals[idx - 1].interval.stop > interval.start
                || (self.intervals[idx - 1].interval.stop == interval.start
                    && self.intervals[idx - 1].interval.is_stop_included
                    && interval.is_start_included))
        {
            result = true;

            if self.intervals[idx - 1].interval.stop > interval.stop
                || (self.intervals[idx - 1].interval.is_stop_included
                    && !interval.is_stop_included
                    && self.intervals[idx - 1].interval.stop == interval.stop)
            {
                // Break the existing interval into two pieces.
                let tail = TimeIntervalData {
                    interval: TimeInterval::new(
                        interval.stop,
                        self.intervals[idx - 1].interval.stop,
                        !interval.is_stop_included,
                        self.intervals[idx - 1].interval.is_stop_included,
                    ),
                    data: self.intervals[idx - 1].data.clone(),
                };
                self.intervals.insert(idx, tail);
            }
            let prev = &self.intervals[idx - 1];
            let truncated = TimeIntervalData {
                interval: TimeInterval::new(
                    prev.interval.start,
                    interval.start,
                    prev.interval.is_start_included,
                    !interval.is_start_included,
                ),
                data: prev.data.clone(),
            };
            self.intervals[idx - 1] = truncated;
        }

        // Keep the start point if interval.start matches but is not included.
        if idx < self.intervals.len()
            && !interval.is_start_included
            && self.intervals[idx].interval.is_start_included
            && interval.start == self.intervals[idx].interval.start
        {
            result = true;
            let point = TimeIntervalData {
                interval: TimeInterval::new(
                    self.intervals[idx].interval.start,
                    self.intervals[idx].interval.start,
                    true,
                    true,
                ),
                data: self.intervals[idx].data.clone(),
            };
            self.intervals.insert(idx, point);
            idx += 1;
        }

        // Remove any intervals completely overlapped by the input interval.
        while idx < self.intervals.len() && interval.stop > self.intervals[idx].interval.stop {
            result = true;
            self.intervals.remove(idx);
        }

        // Handle the case where the input interval ends on the same date as an
        // existing interval.
        if idx < self.intervals.len() && interval.stop == self.intervals[idx].interval.stop {
            result = true;
            if !interval.is_stop_included && self.intervals[idx].interval.is_stop_included {
                // The last point should remain.
                let stop_time = interval.stop;
                let cur = &self.intervals[idx];
                self.intervals[idx] = TimeIntervalData {
                    interval: TimeInterval::new(stop_time, stop_time, true, true),
                    data: cur.data.clone(),
                };
            } else {
                self.intervals.remove(idx);
            }
        }

        // Truncate any partially-overlapped intervals.
        if idx < self.intervals.len()
            && (interval.stop > self.intervals[idx].interval.start
                || (interval.stop == self.intervals[idx].interval.start
                    && interval.is_stop_included
                    && self.intervals[idx].interval.is_start_included))
        {
            result = true;
            let cur = &self.intervals[idx];
            let truncated = TimeIntervalData {
                interval: TimeInterval::new(
                    interval.stop,
                    cur.interval.stop,
                    !interval.is_stop_included,
                    cur.interval.is_stop_included,
                ),
                data: cur.data.clone(),
            };
            self.intervals[idx] = truncated;
        }

        result
    }

    /// Creates a new collection that is the intersection of this collection
    /// and the provided collection.
    ///
    /// Maps to `TimeIntervalCollection.prototype.intersect`.
    pub fn intersect<F>(&self, other: &TimeIntervalCollection<T>, same_data: &F) -> TimeIntervalCollection<T>
    where
        T: Clone,
        F: Fn(&T, &T) -> bool,
    {
        let mut result = TimeIntervalCollection::new();
        let mut left = 0usize;
        let mut right = 0usize;

        while left < self.intervals.len() && right < other.intervals.len() {
            let left_interval = &self.intervals[left];
            let right_interval = &other.intervals[right];

            if left_interval.interval.stop < right_interval.interval.start {
                left += 1;
            } else if right_interval.interval.stop < left_interval.interval.start {
                right += 1;
            } else {
                let same = data_equals(
                    left_interval.data.as_ref(),
                    right_interval.data.as_ref(),
                    same_data,
                );
                if same {
                    if let Some(intersection) =
                        left_interval.interval.intersect(&right_interval.interval)
                    {
                        result.add_interval(
                            TimeIntervalData {
                                interval: intersection,
                                data: left_interval.data.clone(),
                            },
                            same_data,
                        );
                    }
                }

                if left_interval.interval.stop < right_interval.interval.stop
                    || (left_interval.interval.stop == right_interval.interval.stop
                        && !left_interval.interval.is_stop_included
                        && right_interval.interval.is_stop_included)
                {
                    left += 1;
                } else {
                    right += 1;
                }
            }
        }

        result
    }

    /// Compares this collection to another for equality, using `same_data` to
    /// compare interval data.
    ///
    /// Maps to `TimeIntervalCollection.prototype.equals`.
    pub fn equals<F>(&self, other: &TimeIntervalCollection<T>, same_data: &F) -> bool
    where
        F: Fn(&T, &T) -> bool,
    {
        if self.intervals.len() != other.intervals.len() {
            return false;
        }
        for (a, b) in self.intervals.iter().zip(other.intervals.iter()) {
            if a.interval != b.interval {
                return false;
            }
            if !data_equals(a.data.as_ref(), b.data.as_ref(), same_data) {
                return false;
            }
        }
        true
    }
}

/// Compares two optional data payloads. Two `None`s are equal; a `None` and a
/// `Some` are not; two `Some`s are compared with `same_data`.
fn data_equals<T, F>(a: Option<&T>, b: Option<&T>, same_data: &F) -> bool
where
    F: Fn(&T, &T) -> bool,
{
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => same_data(x, y),
        _ => false,
    }
}

fn compare(a: &JulianDate, b: &JulianDate) -> Ordering {
    a.cmp(b)
}

/// Binary search over interval start times. Returns the index of an interval
/// whose start equals `time`, or the bitwise complement of the insertion index.
fn binary_search_start<T>(intervals: &[TimeIntervalData<T>], time: &JulianDate) -> isize {
    let mut low: isize = 0;
    let mut high: isize = intervals.len() as isize - 1;

    while low <= high {
        let mid = (low + high) / 2;
        let mid_start = &intervals[mid as usize].interval.start;
        match mid_start.cmp(time) {
            Ordering::Equal => return mid,
            Ordering::Less => low = mid + 1,
            Ordering::Greater => high = mid - 1,
        }
    }
    !low
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jd(year: i32, month: u32, day: u32, hour: u32) -> JulianDate {
        JulianDate::from_date_components(year, month, day, hour, 0, 0, 0.0)
    }

    fn iv(start: JulianDate, stop: JulianDate, data: i32) -> TimeIntervalData<i32> {
        TimeIntervalData::new(TimeInterval::new(start, stop, true, false), Some(data))
    }

    fn same(a: &i32, b: &i32) -> bool {
        a == b
    }

    #[test]
    fn test_add_and_length() {
        let mut c = TimeIntervalCollection::new();
        assert!(c.is_empty());
        c.add_interval(iv(jd(2012, 8, 1, 0), jd(2012, 8, 1, 6), 1), &same);
        c.add_interval(iv(jd(2012, 8, 1, 6), jd(2012, 8, 1, 12), 2), &same);
        assert_eq!(c.len(), 2);
        assert_eq!(c.start(), Some(jd(2012, 8, 1, 0)));
        assert_eq!(c.stop(), Some(jd(2012, 8, 1, 12)));
        assert!(c.is_start_included());
        assert!(!c.is_stop_included());
    }

    #[test]
    fn test_merge_same_data() {
        let mut c = TimeIntervalCollection::new();
        c.add_interval(iv(jd(2012, 8, 1, 0), jd(2012, 8, 1, 6), 1), &same);
        // Adjacent interval with the same data should merge.
        c.add_interval(iv(jd(2012, 8, 1, 6), jd(2012, 8, 1, 12), 1), &same);
        assert_eq!(c.len(), 1);
        assert_eq!(c.get(0).unwrap().interval.start, jd(2012, 8, 1, 0));
        assert_eq!(c.get(0).unwrap().interval.stop, jd(2012, 8, 1, 12));
    }

    #[test]
    fn test_no_merge_different_data() {
        let mut c = TimeIntervalCollection::new();
        c.add_interval(iv(jd(2012, 8, 1, 0), jd(2012, 8, 1, 6), 1), &same);
        c.add_interval(iv(jd(2012, 8, 1, 6), jd(2012, 8, 1, 12), 2), &same);
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn test_overlapping_new_wins() {
        let mut c = TimeIntervalCollection::new();
        c.add_interval(iv(jd(2012, 8, 1, 0), jd(2012, 8, 1, 12), 1), &same);
        // New interval in the middle with different data truncates the old one.
        c.add_interval(iv(jd(2012, 8, 1, 4), jd(2012, 8, 1, 8), 2), &same);
        assert_eq!(c.len(), 3);
        assert_eq!(c.get(0).unwrap().data, Some(1));
        assert_eq!(c.get(1).unwrap().data, Some(2));
        assert_eq!(c.get(2).unwrap().data, Some(1));
        assert_eq!(c.get(1).unwrap().interval.start, jd(2012, 8, 1, 4));
        assert_eq!(c.get(1).unwrap().interval.stop, jd(2012, 8, 1, 8));
    }

    #[test]
    fn test_index_of_and_contains() {
        let mut c = TimeIntervalCollection::new();
        c.add_interval(iv(jd(2012, 8, 1, 0), jd(2012, 8, 1, 6), 1), &same);
        c.add_interval(iv(jd(2012, 8, 1, 6), jd(2012, 8, 1, 12), 2), &same);

        assert_eq!(c.index_of(&jd(2012, 8, 1, 3)), 0);
        assert_eq!(c.index_of(&jd(2012, 8, 1, 9)), 1);
        assert!(c.contains(&jd(2012, 8, 1, 3)));
        // Stop is exclusive, so 12:00 is not contained.
        assert!(!c.contains(&jd(2012, 8, 1, 12)));
        assert!(!c.contains(&jd(2012, 8, 2, 0)));
    }

    #[test]
    fn test_find_data_for_interval_containing_date() {
        let mut c = TimeIntervalCollection::new();
        c.add_interval(iv(jd(2012, 8, 1, 0), jd(2012, 8, 1, 6), 10), &same);
        c.add_interval(iv(jd(2012, 8, 1, 6), jd(2012, 8, 1, 12), 20), &same);

        assert_eq!(
            c.find_data_for_interval_containing_date(&jd(2012, 8, 1, 3)),
            Some(&10)
        );
        assert_eq!(
            c.find_data_for_interval_containing_date(&jd(2012, 8, 1, 9)),
            Some(&20)
        );
        assert_eq!(
            c.find_data_for_interval_containing_date(&jd(2012, 8, 2, 0)),
            None
        );
    }

    #[test]
    fn test_remove_interval_hole() {
        let mut c = TimeIntervalCollection::new();
        c.add_interval(iv(jd(2012, 8, 1, 0), jd(2012, 8, 1, 12), 1), &same);
        let removed = c.remove_interval(&TimeInterval::new(
            jd(2012, 8, 1, 4),
            jd(2012, 8, 1, 8),
            true,
            true,
        ));
        assert!(removed);
        assert_eq!(c.len(), 2);
        assert!(!c.contains(&jd(2012, 8, 1, 6)));
        assert!(c.contains(&jd(2012, 8, 1, 2)));
        assert!(c.contains(&jd(2012, 8, 1, 10)));
    }

    #[test]
    fn test_remove_all() {
        let mut c = TimeIntervalCollection::new();
        c.add_interval(iv(jd(2012, 8, 1, 0), jd(2012, 8, 1, 6), 1), &same);
        c.remove_all();
        assert!(c.is_empty());
    }

    #[test]
    fn test_intersect() {
        let mut a = TimeIntervalCollection::new();
        a.add_interval(iv(jd(2012, 8, 1, 0), jd(2012, 8, 1, 12), 1), &same);

        let mut b = TimeIntervalCollection::new();
        b.add_interval(iv(jd(2012, 8, 1, 6), jd(2012, 8, 2, 0), 1), &same);

        let inter = a.intersect(&b, &same);
        assert_eq!(inter.len(), 1);
        assert_eq!(inter.get(0).unwrap().interval.start, jd(2012, 8, 1, 6));
        assert_eq!(inter.get(0).unwrap().interval.stop, jd(2012, 8, 1, 12));
    }

    #[test]
    fn test_equals() {
        let mut a = TimeIntervalCollection::new();
        a.add_interval(iv(jd(2012, 8, 1, 0), jd(2012, 8, 1, 6), 1), &same);
        let mut b = TimeIntervalCollection::new();
        b.add_interval(iv(jd(2012, 8, 1, 0), jd(2012, 8, 1, 6), 1), &same);
        assert!(a.equals(&b, &same));

        b.add_interval(iv(jd(2012, 8, 1, 6), jd(2012, 8, 1, 12), 2), &same);
        assert!(!a.equals(&b, &same));
    }

    #[test]
    fn test_empty_interval_ignored() {
        let mut c = TimeIntervalCollection::new();
        let empty = TimeIntervalData::new(
            TimeInterval::new(jd(2012, 8, 1, 6), jd(2012, 8, 1, 0), true, true),
            Some(1),
        );
        c.add_interval(empty, &same);
        assert!(c.is_empty());
    }
}
