//! TimeIntervalCollection - a non-overlapping collection of `TimeInterval`
//! instances sorted by start time.
//!
//! Maps to CesiumJS `Core/TimeIntervalCollection.js`.
//!
//! The collection keeps intervals sorted by start time and guarantees that no
//! two intervals overlap. Adding an interval merges it with adjacent intervals
//! that carry the same data, or splits/truncates existing intervals when the
//! data differs (the newly added interval's data takes precedence).

use crate::gregorian_date::{days_in_month, is_leap_year, GregorianDate};
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
                    let intersection =
                        left_interval.interval.intersect(&right_interval.interval);
                    if !intersection.is_empty() {
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

/// Iso8601.MINIMUM_VALUE equivalent: 0001-01-01T00:00:00Z
fn iso8601_minimum_value() -> JulianDate {
    JulianDate::from_iso8601("0001-01-01T00:00:00Z").unwrap()
}

/// Iso8601.MAXIMUM_VALUE equivalent: 9999-12-31T24:00:00Z
fn iso8601_maximum_value() -> JulianDate {
    JulianDate::from_iso8601("9999-12-31T24:00:00Z").unwrap()
}

/// A duration represented as GregorianDate components.
/// Maps to the scratch GregorianDate used in CesiumJS `parseDuration` / `addToDate`.
#[derive(Debug, Clone, Copy, Default)]
struct Duration {
    year: f64,
    month: f64,
    day: f64,
    hour: f64,
    minute: f64,
    second: f64,
    millisecond: f64,
}

impl Duration {
    fn is_zero(&self) -> bool {
        self.year == 0.0
            && self.month == 0.0
            && self.day == 0.0
            && self.hour == 0.0
            && self.minute == 0.0
            && self.second == 0.0
            && self.millisecond == 0.0
    }
}

/// Parses an ISO8601 duration string (e.g. "P1Y2M3DT1H2M3.5S") or a date-based
/// duration (e.g. "0001-02-03T01:02:03.5").
/// Maps to CesiumJS `parseDuration`.
fn parse_duration(iso8601: Option<&str>) -> Option<Duration> {
    let iso8601 = iso8601?;
    if iso8601.is_empty() {
        return None;
    }

    let mut result = Duration::default();

    if iso8601.starts_with('P') {
        // ISO8601 duration format: P[n]Y[n]M[n]W[n]DT[n]H[n]M[n]S
        let s = &iso8601[1..]; // strip 'P'
        let (date_part, time_part) = if let Some(idx) = s.find('T') {
            (&s[..idx], Some(&s[idx + 1..]))
        } else {
            (s, None)
        };

        // Parse date part: [n]Y[n]M[n]W[n]D
        let mut remaining = date_part;
        while !remaining.is_empty() {
            let num_end = remaining
                .find(|c: char| !c.is_ascii_digit() && c != '.' && c != ',')
                .unwrap_or(remaining.len());
            if num_end == 0 {
                break;
            }
            let num_str = remaining[..num_end].replace(',', ".");
            let num: f64 = num_str.parse().unwrap_or(0.0);
            let designator = remaining.as_bytes()[num_end] as char;
            remaining = &remaining[num_end + 1..];
            match designator {
                'Y' => result.year = num,
                'M' => result.month = num,
                'W' => result.day += num * 7.0,
                'D' => result.day += num,
                _ => {}
            }
        }

        // Parse time part: [n]H[n]M[n]S
        if let Some(tp) = time_part {
            let mut remaining = tp;
            while !remaining.is_empty() {
                let num_end = remaining
                    .find(|c: char| !c.is_ascii_digit() && c != '.' && c != ',')
                    .unwrap_or(remaining.len());
                if num_end == 0 {
                    break;
                }
                let num_str = remaining[..num_end].replace(',', ".");
                let num: f64 = num_str.parse().unwrap_or(0.0);
                let designator = remaining.as_bytes()[num_end] as char;
                remaining = &remaining[num_end + 1..];
                match designator {
                    'H' => result.hour = num,
                    'M' => result.minute = num,
                    'S' => {
                        result.second = num.floor();
                        result.millisecond = (num % 1.0) * 1000.0;
                    }
                    _ => {}
                }
            }
        }
    } else {
        // Date-based duration: parse as a date and extract GregorianDate components
        let s = if iso8601.ends_with('Z') {
            iso8601.to_string()
        } else {
            format!("{}Z", iso8601)
        };
        let jd = JulianDate::from_iso8601(&s)?;
        let g = jd.to_gregorian_date();
        result.year = g.year as f64;
        result.month = g.month as f64;
        result.day = g.day as f64;
        result.hour = g.hour as f64;
        result.minute = g.minute as f64;
        result.second = g.second as f64;
        result.millisecond = g.millisecond;
    }

    if result.is_zero() {
        None
    } else {
        Some(result)
    }
}

/// Adds a duration (represented as GregorianDate components) to a JulianDate.
/// Maps to CesiumJS `addToDate`.
fn add_to_date(julian_date: &JulianDate, duration: &Duration) -> JulianDate {
    let g = julian_date.to_gregorian_date();

    let mut millisecond = g.millisecond + duration.millisecond;
    let mut second = g.second as f64 + duration.second;
    let mut minute = g.minute as f64 + duration.minute;
    let mut hour = g.hour as f64 + duration.hour;
    let mut day = g.day as f64 + duration.day;
    let mut month = g.month as f64 + duration.month;
    let mut year = g.year as f64 + duration.year;

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

    // Adjust days/months/years
    let mut year_i = year as i32;
    let mut month_i = month as u32;
    let mut day_i = day as u32;

    let month_len = |y: i32, m: u32| -> u32 {
        if m >= 1 && m <= 12 {
            days_in_month(y, m)
        } else {
            31
        }
    };

    while day_i > month_len(year_i, month_i) || month_i >= 13 {
        if month_i >= 13 {
            month_i -= 1;
            year_i += (month_i / 12) as i32;
            month_i = month_i % 12;
            month_i += 1;
        }
        if day_i > month_len(year_i, month_i) {
            day_i -= month_len(year_i, month_i);
            month_i += 1;
        }
    }

    let result_g = GregorianDate {
        year: year_i,
        month: month_i,
        day: day_i,
        hour: hour as u32,
        minute: minute as u32,
        second: second as u32,
        millisecond,
        is_leap_second: false,
    };
    JulianDate::from_gregorian_date(&result_g)
}

/// Options for `from_iso8601` and related constructors.
pub struct FromIso8601Options {
    /// The ISO8601 interval string ("start/stop" or "start/stop/duration").
    pub iso8601: String,
    /// Whether the start time is included (default true).
    pub is_start_included: Option<bool>,
    /// Whether the stop time is included (default true).
    pub is_stop_included: Option<bool>,
    /// Add a leading interval from MINIMUM_VALUE to start.
    pub leading_interval: bool,
    /// Add a trailing interval from stop to MAXIMUM_VALUE.
    pub trailing_interval: bool,
}

impl<T> TimeIntervalCollection<T> {
    /// Creates a collection from an ISO8601 interval string.
    /// Maps to `TimeIntervalCollection.fromIso8601`.
    pub fn from_iso8601<F>(options: &FromIso8601Options, same_data: &F) -> Self
    where
        T: Clone + From<usize>,
        F: Fn(&T, &T) -> bool,
    {
        let parts: Vec<&str> = options.iso8601.split('/').collect();
        let start = JulianDate::from_iso8601(parts[0]).unwrap();
        let stop = JulianDate::from_iso8601(parts[1]).unwrap();

        let mut julian_dates: Vec<JulianDate> = Vec::new();

        let duration = if parts.len() > 2 {
            parse_duration(Some(parts[2]))
        } else {
            None
        };

        match duration {
            None => {
                julian_dates.push(start);
                julian_dates.push(stop);
            }
            Some(dur) => {
                let mut date = start;
                julian_dates.push(date);
                while date < stop {
                    date = add_to_date(&date, &dur);
                    if stop <= date {
                        date = stop;
                    }
                    julian_dates.push(date);
                }
            }
        }

        Self::from_julian_date_array(
            &julian_dates,
            options.is_start_included.unwrap_or(true),
            options.is_stop_included.unwrap_or(true),
            options.leading_interval,
            options.trailing_interval,
            same_data,
        )
    }

    /// Creates a collection from an array of JulianDates.
    /// Maps to `TimeIntervalCollection.fromJulianDateArray`.
    pub fn from_julian_date_array<F>(
        julian_dates: &[JulianDate],
        is_start_included: bool,
        is_stop_included: bool,
        leading_interval: bool,
        trailing_interval: bool,
        same_data: &F,
    ) -> Self
    where
        T: Clone + From<usize>,
        F: Fn(&T, &T) -> bool,
    {
        let mut result = Self::new();
        let length = julian_dates.len();
        if length < 2 {
            return result;
        }

        let start_index: usize = if leading_interval { 1 } else { 0 };

        if leading_interval {
            let interval = TimeIntervalData {
                interval: TimeInterval::new(
                    iso8601_minimum_value(),
                    julian_dates[0],
                    true,
                    !is_start_included,
                ),
                data: Some(T::from(result.len())),
            };
            result.add_interval(interval, same_data);
        }

        for i in 0..length - 1 {
            let start_date = julian_dates[i];
            let end_date = julian_dates[i + 1];
            let isi = if result.len() == start_index {
                is_start_included
            } else {
                true
            };
            let isti = if i == length - 2 {
                is_stop_included
            } else {
                false
            };
            let interval = TimeIntervalData {
                interval: TimeInterval::new(start_date, end_date, isi, isti),
                data: Some(T::from(result.len())),
            };
            result.add_interval(interval, same_data);
        }

        if trailing_interval {
            let interval = TimeIntervalData {
                interval: TimeInterval::new(
                    julian_dates[length - 1],
                    iso8601_maximum_value(),
                    !is_stop_included,
                    true,
                ),
                data: Some(T::from(result.len())),
            };
            result.add_interval(interval, same_data);
        }

        result
    }

    /// Creates a collection from an array of ISO8601 duration strings relative to an epoch.
    /// Maps to `TimeIntervalCollection.fromIso8601DurationArray`.
    pub fn from_iso8601_duration_array<F>(
        epoch: &JulianDate,
        iso8601_durations: &[&str],
        relative_to_previous: bool,
        is_start_included: bool,
        is_stop_included: bool,
        leading_interval: bool,
        trailing_interval: bool,
        same_data: &F,
    ) -> Self
    where
        T: Clone + From<usize>,
        F: Fn(&T, &T) -> bool,
    {
        let mut julian_dates: Vec<JulianDate> = Vec::new();
        let mut previous_date: Option<JulianDate> = None;

        for (i, dur_str) in iso8601_durations.iter().enumerate() {
            let dur = parse_duration(Some(dur_str));
            // Allow a duration of 0 on the first iteration (it is just the epoch)
            if dur.is_some() || i == 0 {
                let effective_dur = dur.unwrap_or_default();
                let date = if relative_to_previous {
                    if let Some(prev) = previous_date {
                        add_to_date(&prev, &effective_dur)
                    } else {
                        add_to_date(epoch, &effective_dur)
                    }
                } else {
                    add_to_date(epoch, &effective_dur)
                };
                julian_dates.push(date);
                previous_date = Some(date);
            }
        }

        Self::from_julian_date_array(
            &julian_dates,
            is_start_included,
            is_stop_included,
            leading_interval,
            trailing_interval,
            same_data,
        )
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
