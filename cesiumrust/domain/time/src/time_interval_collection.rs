//! TimeIntervalCollection - 按开始时间排序的、互不重叠的 `TimeInterval`
//! 实例集合。
//!
//! 映射到 CesiumJS `Core/TimeIntervalCollection.js`。
//!
//! 该集合按开始时间排序保存各区间，并保证任意两个区间互不重叠。添加一个
//! 区间时，若其携带相同数据则与相邻区间合并；若数据不同则拆分/截断已有
//! 区间（新添加区间的数据优先）。

// deferred.md #13: `is_leap_year` 目前仅由 julian_date.rs 经全路径调用，本文件暂未使用。
#[allow(unused_imports)]
use crate::gregorian_date::{days_in_month, is_leap_year, GregorianDate};
use crate::julian_date::JulianDate;
use crate::time_interval::TimeInterval;
use std::cmp::Ordering;

/// 一个 `TimeInterval` 及其可选的数据负载。
///
/// 映射到 CesiumJS `TimeInterval`（它携带一个 `data` 属性）。
#[derive(Debug, Clone, PartialEq)]
pub struct TimeIntervalData<T> {
    /// 底层区间（开始/停止/包含标志）。
    pub interval: TimeInterval,
    /// 与该区间关联的数据。
    pub data: Option<T>,
}

impl<T> TimeIntervalData<T> {
    /// 创建一个带数据的新区间。
    pub fn new(interval: TimeInterval, data: Option<T>) -> Self {
        Self { interval, data }
    }

    /// 若区间为空则返回 true。
    pub fn is_empty(&self) -> bool {
        self.interval.is_empty()
    }

    /// 若区间包含给定时间则返回 true。
    pub fn contains(&self, time: &JulianDate) -> bool {
        self.interval.contains(time)
    }
}

/// 按开始时间排序的、互不重叠的 `TimeInterval` 实例集合。
///
/// 映射到 CesiumJS `TimeIntervalCollection`。
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
    /// 创建一个空集合。
    pub fn new() -> Self {
        Self {
            intervals: Vec::new(),
        }
    }

    /// 创建一个以给定区间预填充的集合。
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

    /// 集合中区间数量。
    /// 映射到 `TimeIntervalCollection.prototype.length`。
    pub fn len(&self) -> usize {
        self.intervals.len()
    }

    /// 若集合为空则返回 true。
    /// 映射到 `TimeIntervalCollection.prototype.isEmpty`。
    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    /// 集合的开始时间（第一个区间的开始）。
    /// 映射到 `TimeIntervalCollection.prototype.start`。
    pub fn start(&self) -> Option<JulianDate> {
        self.intervals.first().map(|i| i.interval.start)
    }

    /// 开始时间是否包含在集合内。
    /// 映射到 `TimeIntervalCollection.prototype.isStartIncluded`。
    pub fn is_start_included(&self) -> bool {
        self.intervals
            .first()
            .map(|i| i.interval.is_start_included)
            .unwrap_or(false)
    }

    /// 集合的停止时间（最后一个区间的停止）。
    /// 映射到 `TimeIntervalCollection.prototype.stop`。
    pub fn stop(&self) -> Option<JulianDate> {
        self.intervals.last().map(|i| i.interval.stop)
    }

    /// 停止时间是否包含在集合内。
    /// 映射到 `TimeIntervalCollection.prototype.isStopIncluded`。
    pub fn is_stop_included(&self) -> bool {
        self.intervals
            .last()
            .map(|i| i.interval.is_stop_included)
            .unwrap_or(false)
    }

    /// 获取指定索引处的区间。
    /// 映射到 `TimeIntervalCollection.prototype.get`。
    pub fn get(&self, index: usize) -> Option<&TimeIntervalData<T>> {
        self.intervals.get(index)
    }

    /// 返回遍历各区间的迭代器。
    pub fn iter(&self) -> std::slice::Iter<'_, TimeIntervalData<T>> {
        self.intervals.iter()
    }

    /// 从集合中移除所有区间。
    /// 映射到 `TimeIntervalCollection.prototype.removeAll`。
    pub fn remove_all(&mut self) {
        self.intervals.clear();
    }

    /// 查找并返回包含指定日期的区间的索引。当没有区间包含该日期时，
    /// 返回一个负数（插入索引的按位取反），与 CesiumJS 语义一致。
    ///
    /// 映射到 `TimeIntervalCollection.prototype.indexOf`。
    pub fn index_of(&self, date: &JulianDate) -> isize {
        let intervals = &self.intervals;

        // 对开始时间进行二分查找，寻找 start == date 的区间。
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

    /// 若集合包含指定日期则返回 true。
    /// 映射到 `TimeIntervalCollection.prototype.contains`。
    pub fn contains(&self, date: &JulianDate) -> bool {
        self.index_of(date) >= 0
    }

    /// 查找并返回包含指定日期的区间。
    /// 映射到 `TimeIntervalCollection.prototype.findIntervalContainingDate`。
    pub fn find_interval_containing_date(&self, date: &JulianDate) -> Option<&TimeIntervalData<T>> {
        let index = self.index_of(date);
        if index >= 0 {
            self.intervals.get(index as usize)
        } else {
            None
        }
    }

    /// 查找并返回包含指定日期的区间的数据。
    /// 映射到 `TimeIntervalCollection.prototype.findDataForIntervalContainingDate`。
    pub fn find_data_for_interval_containing_date(&self, date: &JulianDate) -> Option<&T> {
        self.find_interval_containing_date(date)
            .and_then(|i| i.data.as_ref())
    }

    /// 返回匹配可选 start/stop/inclusion 参数的第一个区间。
    /// 为 `None` 的参数视为不关心。
    ///
    /// 映射到 `TimeIntervalCollection.prototype.findInterval`。
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

    /// 向集合添加一个区间，合并包含相同数据的区间，并在需要时拆分数据
    /// 不同的区间，以维持一个互不重叠的集合。新区间中的数据优先于任何
    /// 已有区间。
    ///
    /// `same_data` 比较两个数据负载，以决定相邻区间能否合并。
    ///
    /// 映射到 `TimeIntervalCollection.prototype.addInterval`。
    pub fn add_interval<F>(&mut self, mut interval: TimeIntervalData<T>, same_data: &F)
    where
        T: Clone,
        F: Fn(&T, &T) -> bool,
    {
        if interval.is_empty() {
            return;
        }

        // 快速路径：追加在所有已有内容之后。
        if self.intervals.is_empty()
            || interval.interval.start > self.intervals[self.intervals.len() - 1].interval.stop
        {
            self.intervals.push(interval);
            return;
        }

        // 保持列表按开始日期排序。
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
            // 查看前一个区间是否与此区间重叠。
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
                    // 重叠的区间具有相同数据，因此将它们合并。
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
                    // 数据不同：新区间胜出；截断前一个区间，
                    // 若其延伸超过新区间则拆分它。
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
            // 查看此区间之后的区间是否与此区间重叠。
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
                    // 相同数据：将它们合并。
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
                    // 数据不同：新区间胜出；截断下一个区间。
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
                        // 找到部分跨越；下一个区间无法被跨越。
                        break;
                    }
                }
            } else {
                // 找到我们跨越的最后一个区间；停止查找。
                break;
            }
        }

        self.intervals.insert(idx, interval);
    }

    /// 从本集合移除指定区间，在指定区间处创建一个空洞。输入的区间数据被忽略。
    /// 若区间的任何部分原本在集合中则返回 true。
    ///
    /// 映射到 `TimeIntervalCollection.prototype.removeInterval`。
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

        // 检查前一个区间末尾的截断。
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
                // 将已有区间拆成两段。
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

        // 若 interval.start 匹配但不包含，则保留该起始点。
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

        // 移除被输入区间完全覆盖的所有区间。
        while idx < self.intervals.len() && interval.stop > self.intervals[idx].interval.stop {
            result = true;
            self.intervals.remove(idx);
        }

        // 处理输入区间与某个已有区间
        // 结束于同一日期的情况。
        if idx < self.intervals.len() && interval.stop == self.intervals[idx].interval.stop {
            result = true;
            if !interval.is_stop_included && self.intervals[idx].interval.is_stop_included {
                // 最后一个点应当保留。
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

        // 截断任何部分重叠的区间。
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

    /// 创建一个新集合，为本集合与所提供集合的交集。
    ///
    /// 映射到 `TimeIntervalCollection.prototype.intersect`。
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

    /// 将本集合与另一个集合比较是否相等，使用 `same_data` 比较区间数据。
    ///
    /// 映射到 `TimeIntervalCollection.prototype.equals`。
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

/// Iso8601.MINIMUM_VALUE 的等价：0001-01-01T00:00:00Z
fn iso8601_minimum_value() -> JulianDate {
    JulianDate::from_iso8601("0001-01-01T00:00:00Z").unwrap()
}

/// Iso8601.MAXIMUM_VALUE 的等价：9999-12-31T24:00:00Z
fn iso8601_maximum_value() -> JulianDate {
    JulianDate::from_iso8601("9999-12-31T24:00:00Z").unwrap()
}

/// 以 GregorianDate 分量表示的时长。
/// 映射到 CesiumJS `parseDuration` / `addToDate` 中使用的临时 GregorianDate。
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

/// 解析 ISO8601 时长字符串（如 "P1Y2M3DT1H2M3.5S"）或基于日期的
/// 时长（如 "0001-02-03T01:02:03.5"）。
/// 映射到 CesiumJS `parseDuration`。
fn parse_duration(iso8601: Option<&str>) -> Option<Duration> {
    let iso8601 = iso8601?;
    if iso8601.is_empty() {
        return None;
    }

    let mut result = Duration::default();

    // deferred.md #13: 手动 strip 'P' 前缀，等价 strip_prefix('P')；风格问题非逻辑错误。
    #[allow(clippy::manual_strip)]
    if iso8601.starts_with('P') {
        // ISO8601 时长格式：P[n]Y[n]M[n]W[n]DT[n]H[n]M[n]S
        let s = &iso8601[1..]; // 去掉 'P'
        let (date_part, time_part) = if let Some(idx) = s.find('T') {
            (&s[..idx], Some(&s[idx + 1..]))
        } else {
            (s, None)
        };

        // 解析日期部分：[n]Y[n]M[n]W[n]D
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

        // 解析时间部分：[n]H[n]M[n]S
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
        // 基于日期的时长：解析为日期并提取 GregorianDate 各分量
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

/// 将一个时长（以 GregorianDate 各分量表示）加到一个 JulianDate 上。
/// 映射到 CesiumJS `addToDate`。
fn add_to_date(julian_date: &JulianDate, duration: &Duration) -> JulianDate {
    let g = julian_date.to_gregorian_date();

    let mut millisecond = g.millisecond + duration.millisecond;
    let mut second = g.second as f64 + duration.second;
    let mut minute = g.minute as f64 + duration.minute;
    let mut hour = g.hour as f64 + duration.hour;
    let mut day = g.day as f64 + duration.day;
    // deferred.md #13: month/year 之后未被重新赋值（改用 month_i/year_i），mut 冗余。
    #[allow(unused_mut)]
    let mut month = g.month as f64 + duration.month;
    #[allow(unused_mut)]
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

    // 调整日/月/年
    let mut year_i = year as i32;
    let mut month_i = month as u32;
    let mut day_i = day as u32;

    // deferred.md #13: 手动范围判断 (m >= 1 && m <= 12)，等价 (1..=12).contains(&m)。
    #[allow(clippy::manual_range_contains)]
    let month_len = |y: i32, m: u32| -> u32 {
        if m >= 1 && m <= 12 {
            days_in_month(y, m)
        } else {
            31
        }
    };

    while day_i > month_len(year_i, month_i) || month_i >= 13 {
        // deferred.md #13: month_i = month_i % 12 可用 %= 简写（属性置于 if 块，赋值语句不支持属性）。
        #[allow(clippy::assign_op_pattern)]
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

/// `from_iso8601` 及相关构造函数的选项。
pub struct FromIso8601Options {
    /// ISO8601 区间字符串（"start/stop" 或 "start/stop/duration"）。
    pub iso8601: String,
    /// 是否包含开始时间（默认 true）。
    pub is_start_included: Option<bool>,
    /// 是否包含停止时间（默认 true）。
    pub is_stop_included: Option<bool>,
    /// 添加一个从 MINIMUM_VALUE 到 start 的前置区间。
    pub leading_interval: bool,
    /// 添加一个从 stop 到 MAXIMUM_VALUE 的后置区间。
    pub trailing_interval: bool,
}

impl<T> TimeIntervalCollection<T> {
    /// 从 ISO8601 区间字符串创建集合。
    /// 映射到 `TimeIntervalCollection.fromIso8601`。
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

    /// 从 JulianDate 数组创建集合。
    /// 映射到 `TimeIntervalCollection.fromJulianDateArray`。
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

    /// 从一个相对于纪元的 ISO8601 时长字符串数组创建集合。
    /// 映射到 `TimeIntervalCollection.fromIso8601DurationArray`。
    // deferred.md #13: 参数 8/7，保持与 CesiumJS fromIso8601DurationArray 签名一一对应。
    #[allow(clippy::too_many_arguments)]
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
            // 允许首次迭代时时长为 0（它仅是纪元）
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

/// 比较两个可选数据负载。两个 `None` 视为相等；一个 `None` 与一个
/// `Some` 视为不等；两个 `Some` 用 `same_data` 比较。
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

/// 对各区间的开始时间进行二分查找。返回 start 等于 `time` 的区间索引，
/// 或插入索引的按位取反。
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
        // 携带相同数据的相邻区间应当合并。
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
        // 中间插入的数据不同的新区间会截断旧区间。
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
        // 停止时间为开区间，因此 12:00 不被包含。
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
