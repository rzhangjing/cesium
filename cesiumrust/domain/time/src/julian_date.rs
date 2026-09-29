//! JulianDate - 天文儒略日表示。
//! 映射到 CesiumJS `Core/JulianDate.js`
//!
//! 以 dayNumber + secondsOfDay 存储时间（内部采用 TAI 标准）。
//! 儒略日是自 -4712 年（即公元前 4713 年）1 月 1 日正午起经过的天数。

use crate::gregorian_date::GregorianDate;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// 用于换算的时间常量。
/// 映射到 CesiumJS `Core/TimeConstants.js`
pub mod constants {
    pub const SECONDS_PER_MILLISECOND: f64 = 0.001;
    pub const SECONDS_PER_MINUTE: f64 = 60.0;
    pub const MINUTES_PER_HOUR: f64 = 60.0;
    pub const HOURS_PER_DAY: f64 = 24.0;
    pub const SECONDS_PER_HOUR: f64 = 3600.0;
    pub const MINUTES_PER_DAY: f64 = 1440.0;
    pub const SECONDS_PER_DAY: f64 = 86400.0;
    pub const DAYS_PER_JULIAN_CENTURY: f64 = 36525.0;
    pub const MODIFIED_JULIAN_DATE_DIFFERENCE: f64 = 2400000.5;
}

use constants::*;

/// 用于表示日期的时间标准。
/// 映射到 CesiumJS `TimeStandard`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TimeStandard {
    /// 协调世界时。
    #[default]
    UTC,
    /// 国际原子时。
    TAI,
}

/// 一个闰秒条目：其发生的 TAI 儒略日期以及累计的 TAI-UTC 偏移。
#[derive(Debug, Clone, Copy)]
struct LeapSecond {
    /// 该闰秒发生时对应的 TAI 儒略日期 (day_number, seconds_of_day)。
    day_number: i64,
    seconds_of_day: f64,
    /// 该闰秒之后累计的 TAI-UTC 偏移（秒）。
    offset: f64,
}

/// Cesium 各处使用的闰秒表。
/// 映射到 CesiumJS `JulianDate.leapSeconds`
const LEAP_SECONDS: &[LeapSecond] = &[
    LeapSecond { day_number: 2441317, seconds_of_day: 43210.0, offset: 10.0 }, // 1972-01-01
    LeapSecond { day_number: 2441499, seconds_of_day: 43211.0, offset: 11.0 }, // 1972-07-01
    LeapSecond { day_number: 2441683, seconds_of_day: 43212.0, offset: 12.0 }, // 1973-01-01
    LeapSecond { day_number: 2442048, seconds_of_day: 43213.0, offset: 13.0 }, // 1974-01-01
    LeapSecond { day_number: 2442413, seconds_of_day: 43214.0, offset: 14.0 }, // 1975-01-01
    LeapSecond { day_number: 2442778, seconds_of_day: 43215.0, offset: 15.0 }, // 1976-01-01
    LeapSecond { day_number: 2443144, seconds_of_day: 43216.0, offset: 16.0 }, // 1977-01-01
    LeapSecond { day_number: 2443509, seconds_of_day: 43217.0, offset: 17.0 }, // 1978-01-01
    LeapSecond { day_number: 2443874, seconds_of_day: 43218.0, offset: 18.0 }, // 1979-01-01
    LeapSecond { day_number: 2444239, seconds_of_day: 43219.0, offset: 19.0 }, // 1980-01-01
    LeapSecond { day_number: 2444786, seconds_of_day: 43220.0, offset: 20.0 }, // 1981-07-01
    LeapSecond { day_number: 2445151, seconds_of_day: 43221.0, offset: 21.0 }, // 1982-07-01
    LeapSecond { day_number: 2445516, seconds_of_day: 43222.0, offset: 22.0 }, // 1983-07-01
    LeapSecond { day_number: 2446247, seconds_of_day: 43223.0, offset: 23.0 }, // 1985-07-01
    LeapSecond { day_number: 2447161, seconds_of_day: 43224.0, offset: 24.0 }, // 1988-01-01
    LeapSecond { day_number: 2447892, seconds_of_day: 43225.0, offset: 25.0 }, // 1990-01-01
    LeapSecond { day_number: 2448257, seconds_of_day: 43226.0, offset: 26.0 }, // 1991-01-01
    LeapSecond { day_number: 2448804, seconds_of_day: 43227.0, offset: 27.0 }, // 1992-07-01
    LeapSecond { day_number: 2449169, seconds_of_day: 43228.0, offset: 28.0 }, // 1993-07-01
    LeapSecond { day_number: 2449534, seconds_of_day: 43229.0, offset: 29.0 }, // 1994-07-01
    LeapSecond { day_number: 2450083, seconds_of_day: 43230.0, offset: 30.0 }, // 1996-01-01
    LeapSecond { day_number: 2450630, seconds_of_day: 43231.0, offset: 31.0 }, // 1997-07-01
    LeapSecond { day_number: 2451179, seconds_of_day: 43232.0, offset: 32.0 }, // 1999-01-01
    LeapSecond { day_number: 2453736, seconds_of_day: 43233.0, offset: 33.0 }, // 2006-01-01
    LeapSecond { day_number: 2454832, seconds_of_day: 43234.0, offset: 34.0 }, // 2009-01-01
    LeapSecond { day_number: 2456109, seconds_of_day: 43235.0, offset: 35.0 }, // 2012-07-01
    LeapSecond { day_number: 2457204, seconds_of_day: 43236.0, offset: 36.0 }, // 2015-07-01
    LeapSecond { day_number: 2457754, seconds_of_day: 43237.0, offset: 37.0 }, // 2017-01-01
];

/// 二分查找闰秒索引。返回其 TAI 日期 >= 给定 (day_number, seconds_of_day)
/// 的闰秒索引，或插入点。
fn find_leap_second_index(day_number: i64, seconds_of_day: f64) -> usize {
    let mut lo = 0usize;
    let mut hi = LEAP_SECONDS.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        let ls = &LEAP_SECONDS[mid];
        let cmp = if ls.day_number < day_number
            || (ls.day_number == day_number && ls.seconds_of_day < seconds_of_day)
        {
            Ordering::Less
        } else if ls.day_number == day_number
            && (ls.seconds_of_day - seconds_of_day).abs() < 1e-10
        {
            Ordering::Equal
        } else {
            Ordering::Greater
        };
        match cmp {
            Ordering::Less => lo = mid + 1,
            Ordering::Equal => return mid,
            Ordering::Greater => hi = mid,
        }
    }
    lo // 插入点
}

/// 将一个 JulianDate 从 UTC 就地转换为 TAI。
/// 映射到 CesiumJS `convertUtcToTai`
fn convert_utc_to_tai(day_number: &mut i64, seconds_of_day: &mut f64) {
    let mut index = find_leap_second_index(*day_number, *seconds_of_day);

    if index >= LEAP_SECONDS.len() {
        index = LEAP_SECONDS.len() - 1;
    }

    let mut offset = LEAP_SECONDS[index].offset;
    if index > 0 {
        // 检查是否差了一个：闰秒的 TAI 日期与我们的 UTC 日期（当作 TAI）
        // 之间的差值不应超过该偏移。
        let ls = &LEAP_SECONDS[index];
        let difference = (ls.day_number as f64 - *day_number as f64) * SECONDS_PER_DAY
            + (ls.seconds_of_day - *seconds_of_day);
        if difference > offset {
            index -= 1;
            offset = LEAP_SECONDS[index].offset;
        }
    }

    // 加上偏移秒数
    *seconds_of_day += offset;
    // 归一化
    let extra_days = (*seconds_of_day / SECONDS_PER_DAY) as i64;
    *day_number += extra_days;
    *seconds_of_day -= SECONDS_PER_DAY * extra_days as f64;
    if *seconds_of_day < 0.0 {
        *day_number -= 1;
        *seconds_of_day += SECONDS_PER_DAY;
    }
}

/// 将一个 TAI JulianDate 转换为 UTC。若该日期落在闰秒期间
/// （转换存在歧义）则返回 None。
/// 映射到 CesiumJS `convertTaiToUtc`
fn convert_tai_to_utc(day_number: i64, seconds_of_day: f64) -> Option<(i64, f64)> {
    let index = find_leap_second_index(day_number, seconds_of_day);

    // 第一个闰秒之前的所有时间都使用第一个偏移。
    if index == 0 {
        let offset = LEAP_SECONDS[0].offset;
        return Some(apply_offset(day_number, seconds_of_day, -offset));
    }

    // 最后一个闰秒之后的所有时间都使用最后一个偏移。
    if index >= LEAP_SECONDS.len() {
        let offset = LEAP_SECONDS[LEAP_SECONDS.len() - 1].offset;
        return Some(apply_offset(day_number, seconds_of_day, -offset));
    }

    // 计算找到的闰秒与我们正在转换的时间之间的差值。
    let ls = &LEAP_SECONDS[index];
    let difference = (ls.day_number as f64 - day_number as f64) * SECONDS_PER_DAY
        + (ls.seconds_of_day - seconds_of_day);

    if difference.abs() < 1e-10 {
        // 该日期恰好位于闰秒表条目处。
        let offset = LEAP_SECONDS[index].offset;
        return Some(apply_offset(day_number, seconds_of_day, -offset));
    }

    if difference <= 1.0 {
        // 请求的日期正处于闰秒的那一刻，无法转换为 UTC。
        return None;
    }

    // 该时间位于两个闰秒之间；使用上一个闰秒的偏移。
    let offset = LEAP_SECONDS[index - 1].offset;
    Some(apply_offset(day_number, seconds_of_day, -offset))
}

/// 辅助函数：向 (day_number, seconds_of_day) 施加一个秒数偏移并归一化。
fn apply_offset(day_number: i64, seconds_of_day: f64, offset: f64) -> (i64, f64) {
    let mut sod = seconds_of_day + offset;
    let mut dn = day_number;
    let extra_days = (sod / SECONDS_PER_DAY) as i64;
    dn += extra_days;
    sod -= SECONDS_PER_DAY * extra_days as f64;
    if sod < 0.0 {
        dn -= 1;
        sod += SECONDS_PER_DAY;
    }
    (dn, sod)
}

/// 表示一个天文儒略日。
/// 映射到 CesiumJS `JulianDate`
///
/// 为提高精度，将日期的整数部分与秒数部分分开存储。内部以 TAI 存储。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct JulianDate {
    /// 整天数。
    pub day_number: i64,
    /// 当前天内的秒数。
    pub seconds_of_day: f64,
}

impl JulianDate {
    /// 从儒略日数与当日秒数创建一个新的 JulianDate。
    /// 映射到 `new JulianDate(julianDayNumber, secondsOfDay, timeStandard)`
    ///
    /// 若 `time_standard` 为 UTC（默认），则在内部转换为 TAI。
    pub fn new(julian_day_number: f64, seconds_of_day: f64) -> Self {
        Self::with_time_standard(julian_day_number, seconds_of_day, TimeStandard::UTC)
    }

    /// 使用显式时间标准创建一个新的 JulianDate。
    pub fn with_time_standard(
        julian_day_number: f64,
        seconds_of_day: f64,
        time_standard: TimeStandard,
    ) -> Self {
        let whole_days = julian_day_number as i64;
        let sod = seconds_of_day + (julian_day_number - whole_days as f64) * SECONDS_PER_DAY;
        let mut result = Self::set_components(whole_days, sod);

        if time_standard == TimeStandard::UTC {
            convert_utc_to_tai(&mut result.day_number, &mut result.seconds_of_day);
        }
        result
    }

    /// 从日期分量创建 JulianDate（假定 UTC，含闰秒校正）。
    /// 映射到 `JulianDate.fromGregorianDate` / `computeJulianDateComponents`
    pub fn from_date_components(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
        millisecond: f64,
    ) -> Self {
        let (day_number, seconds_of_day) =
            compute_julian_date_components(year, month, day, hour, minute, second, millisecond);
        let mut result = Self::set_components(day_number, seconds_of_day);
        // from_date_components 为 UTC，转换为 TAI
        convert_utc_to_tai(&mut result.day_number, &mut result.seconds_of_day);
        result
    }

    /// 从 GregorianDate 创建 JulianDate。
    pub fn from_gregorian_date(date: &GregorianDate) -> Self {
        Self::from_date_components(
            date.year,
            date.month,
            date.day,
            date.hour,
            date.minute,
            date.second,
            date.millisecond,
        )
    }

    /// 从 ISO 8601 日期字符串创建 JulianDate。
    /// 映射到 `JulianDate.fromIso8601`
    ///
    /// 支持：日历日期（基本/扩展格式）、序数日期、周日期、
    /// 带小数秒的时间、UTC 偏移（Z/±HH/±HH:MM）、闰秒（second=60），
    /// 以及 24:00:00 午夜表示法。
    ///
    /// 若字符串不是有效的 ISO 8601 日期则返回 None。
    pub fn from_iso8601(iso8601_string: &str) -> Option<Self> {
        // 根据 ISO 8601，逗号和圆点都表示小数
        let iso8601_string = iso8601_string.replace(',', ".");

        // 按必需的 'T' 拆分为日期与时间分量
        let parts: Vec<&str> = iso8601_string.splitn(2, 'T').collect();
        let date_str = parts[0];
        let time_str = parts.get(1).copied();

        if date_str.is_empty() {
            return None;
        }

        let mut year: i32;
        let mut month: i32 = 1;
        let mut day: i32 = 1;
        let mut hour: i32 = 0;
        let mut minute: i32 = 0;
        let mut second: f64 = 0.0;
        let mut millisecond: f64 = 0.0;

        // 解析日期分量
        if let Some((y, m, d)) = parse_calendar_date(date_str) {
            year = y;
            month = m;
            day = d;
        } else if let Some((y, m)) = parse_calendar_month(date_str) {
            year = y;
            month = m;
        } else if let Some(y) = parse_calendar_year(date_str) {
            year = y;
        } else if let Some((y, doy)) = parse_ordinal_date(date_str) {
            year = y;
            let in_leap_year = crate::gregorian_date::is_leap_year(y);
            if doy < 1 || (in_leap_year && doy > 366) || (!in_leap_year && doy > 365) {
                return None;
            }
            // 将序数日期转换为月/日
            let (m, d) = ordinal_to_month_day(y, doy);
            month = m;
            day = d;
        } else if let Some((y, w, d)) = parse_week_date(date_str) {
            year = y;
            // 将 ISO 周日期转换为序数日期
            let jan4_weekday = day_of_week(y, 1, 4); // 0=周日
            let day_of_year = w * 7 + d - jan4_weekday - 3;
            let (m, dd) = ordinal_to_month_day(y, day_of_year);
            month = m;
            day = dd;
        } else {
            return None;
        }

        // 校验日期分量
        // deferred.md #13: 手动范围判断，等价 !(1..=12).contains(&month)。
        #[allow(clippy::manual_range_contains)]
        if month < 1 || month > 12 || day < 1 {
            return None;
        }
        let in_leap_year = crate::gregorian_date::is_leap_year(year);
        let days_in_month_arr: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let max_day = if month == 2 && in_leap_year { 29 } else { days_in_month_arr[(month - 1) as usize] };
        if day > max_day {
            return None;
        }

        // Parse time component
        if let Some(time) = time_str {
            let parsed = parse_time(time)?;
            hour = parsed.0;
            minute = parsed.1;
            second = parsed.2;
            millisecond = parsed.3;

            // 校验时间范围
            if minute >= 60 || second >= 61.0 || hour > 24
                || (hour == 24 && (minute > 0 || second > 0.0 || millisecond > 0.0))
            {
                return None;
            }

            // 应用 UTC 偏移
            let offset = parsed.4;
            let offset_hours = parsed.5;
            let offset_minutes = parsed.6;
            match offset {
                '+' => {
                    hour -= offset_hours;
                    minute -= offset_minutes;
                }
                '-' => {
                    hour += offset_hours;
                    minute += offset_minutes;
                }
                'Z' => {} // UTC，无需调整
                _ => {} // 未指定偏移 - 视为 UTC（Rust 没有本地时区概念）
            }
        }

        // ISO8601 用 second=60 表示闰秒。
        // 暂时减去一秒以构造有效的 UTC 日期，然后在 TAI 转换后加回。
        let is_leap_second = second == 60.0;
        if is_leap_second {
            second -= 1.0;
        }

        // 在应用 UTC 偏移后归一化
        while minute >= 60 {
            minute -= 60;
            hour += 1;
        }
        while hour >= 24 {
            hour -= 24;
            day += 1;
        }

        let mut tmp_max_day = if in_leap_year && month == 2 { 29 } else { days_in_month_arr[(month - 1) as usize] };
        while day > tmp_max_day {
            day -= tmp_max_day;
            month += 1;
            if month > 12 {
                month -= 12;
                year += 1;
            }
            tmp_max_day = if crate::gregorian_date::is_leap_year(year) && month == 2 { 29 } else { days_in_month_arr[(month - 1) as usize] };
        }

        while minute < 0 {
            minute += 60;
            hour -= 1;
        }
        while hour < 0 {
            hour += 24;
            day -= 1;
        }
        while day < 1 {
            month -= 1;
            if month < 1 {
                month += 12;
                year -= 1;
            }
            tmp_max_day = if crate::gregorian_date::is_leap_year(year) && month == 2 { 29 } else { days_in_month_arr[(month - 1) as usize] };
            day += tmp_max_day;
        }

        // 从分量创建 JulianDate（UTC）
        let (dn, sod) = compute_julian_date_components(
            year,
            month as u32,
            day as u32,
            hour as u32,
            minute as u32,
            second as u32,
            millisecond + (second - second as u32 as f64) * 1000.0,
        );
        let mut result = Self::set_components(dn, sod);
        convert_utc_to_tai(&mut result.day_number, &mut result.seconds_of_day);

        // 若我们处于闰秒，则将其加回
        if is_leap_second {
            result = result.add_seconds(1.0);
        }

        Some(result)
    }

    /// 计算给定实例领先 UTC 的秒数。
    /// 映射到 `JulianDate.computeTaiMinusUtc`
    pub fn compute_tai_minus_utc(&self) -> f64 {
        let insertion_or_match = find_leap_second_index(self.day_number, self.seconds_of_day);

        // 判断这是精确匹配还是插入点
        let is_exact_match = insertion_or_match < LEAP_SECONDS.len()
            && LEAP_SECONDS[insertion_or_match].day_number == self.day_number
            && (LEAP_SECONDS[insertion_or_match].seconds_of_day - self.seconds_of_day).abs() < 1e-10;

        let index = if is_exact_match {
            insertion_or_match
        } else {
            // insertion_or_match 是插入点（第一个 > 该日期的条目）
            // CesiumJS: index = ~index; --index; => insertion_point - 1
            if insertion_or_match == 0 { 0 } else { insertion_or_match - 1 }
        };

        LEAP_SECONDS[index].offset
    }

    /// 将此 TAI 日期转换为 UTC 分量 (day_number, seconds_of_day)。
    /// 若该日期落在闰秒期间则返回 None。
    pub fn to_utc_components(&self) -> Option<(i64, f64)> {
        convert_tai_to_utc(self.day_number, self.seconds_of_day)
    }

    /// 创建表示当前系统时间的 JulianDate。
    /// 映射到 `JulianDate.now()`
    pub fn now() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let unix_seconds = now.as_secs_f64();
        // Unix 纪元（1970-01-01）= 儒略日 2440587.5
        let julian_days = unix_seconds / SECONDS_PER_DAY + 2440587.5;
        Self::new(julian_days, 0.0)
    }

    /// 从自 Unix 纪元起的秒数创建 JulianDate。
    pub fn from_unix_seconds(unix_seconds: f64) -> Self {
        let julian_days = unix_seconds / SECONDS_PER_DAY + 2440587.5;
        Self::new(julian_days, 0.0)
    }

    /// 转换为自 Unix 纪元起的秒数（基于 UTC）。
    /// 映射到 CesiumJS `JulianDate.toDate` → `Date.getTime() / 1000`
    pub fn to_unix_seconds(&self) -> f64 {
        let g = self.to_gregorian_date();
        let second = if g.is_leap_second { g.second - 1 } else { g.second };
        let (dn, sod) = compute_julian_date_components(
            g.year, g.month, g.day, g.hour, g.minute, second, g.millisecond,
        );
        // 这些是 UTC 分量；转换为总天数并减去 Unix 纪元
        let extra_days = (sod / SECONDS_PER_DAY) as i64;
        let mut day_number = dn + extra_days;
        let mut seconds_of_day = sod - SECONDS_PER_DAY * extra_days as f64;
        if seconds_of_day < 0.0 {
            day_number -= 1;
            seconds_of_day += SECONDS_PER_DAY;
        }
        let total_utc_days = day_number as f64 + seconds_of_day / SECONDS_PER_DAY;
        (total_utc_days - 2440587.5) * SECONDS_PER_DAY
    }

    /// 归一化 day_number 与 seconds_of_day 分量。
    /// 映射到 `setComponents`
    fn set_components(whole_days: i64, seconds_of_day: f64) -> Self {
        let extra_days = (seconds_of_day / SECONDS_PER_DAY) as i64;
        let mut day_number = whole_days + extra_days;
        let mut sod = seconds_of_day - SECONDS_PER_DAY * extra_days as f64;

        if sod < 0.0 {
            day_number -= 1;
            sod += SECONDS_PER_DAY;
        }

        Self {
            day_number,
            seconds_of_day: sod,
        }
    }

    /// 计算整数与小数天的总数。
    /// 映射到 `JulianDate.totalDays`
    pub fn total_days(&self) -> f64 {
        self.day_number as f64 + self.seconds_of_day / SECONDS_PER_DAY
    }

    /// 计算两个日期之间的秒数差（left - right）。
    /// 映射到 `JulianDate.secondsDifference`
    pub fn seconds_difference(&self, other: &Self) -> f64 {
        let day_diff = (self.day_number - other.day_number) as f64 * SECONDS_PER_DAY;
        day_diff + (self.seconds_of_day - other.seconds_of_day)
    }

    /// 计算两个日期之间的天数差（left - right）。
    /// 映射到 `JulianDate.daysDifference`
    pub fn days_difference(&self, other: &Self) -> f64 {
        let day_diff = (self.day_number - other.day_number) as f64;
        let second_diff = (self.seconds_of_day - other.seconds_of_day) / SECONDS_PER_DAY;
        day_diff + second_diff
    }

    /// 向此日期加上秒数。
    /// 映射到 `JulianDate.addSeconds`
    pub fn add_seconds(&self, seconds: f64) -> Self {
        Self::set_components(self.day_number, self.seconds_of_day + seconds)
    }

    /// 向此日期加上分钟数。
    /// 映射到 `JulianDate.addMinutes`
    pub fn add_minutes(&self, minutes: f64) -> Self {
        Self::set_components(
            self.day_number,
            self.seconds_of_day + minutes * SECONDS_PER_MINUTE,
        )
    }

    /// 向此日期加上小时数。
    /// 映射到 `JulianDate.addHours`
    pub fn add_hours(&self, hours: f64) -> Self {
        Self::set_components(
            self.day_number,
            self.seconds_of_day + hours * SECONDS_PER_HOUR,
        )
    }

    /// 向此日期加上天数。
    /// 映射到 `JulianDate.addDays`
    pub fn add_days(&self, days: f64) -> Self {
        let extra_days = days as i64;
        let remaining_seconds = (days - extra_days as f64) * SECONDS_PER_DAY;
        Self::set_components(
            self.day_number + extra_days,
            self.seconds_of_day + remaining_seconds,
        )
    }

    /// 若此日期早于另一个则返回 true。
    /// 映射到 `JulianDate.lessThan`
    pub fn less_than(&self, other: &Self) -> bool {
        self.day_number < other.day_number
            || (self.day_number == other.day_number
                && self.seconds_of_day < other.seconds_of_day)
    }

    /// 若此日期晚于另一个则返回 true。
    /// 映射到 `JulianDate.greaterThan`
    pub fn greater_than(&self, other: &Self) -> bool {
        self.day_number > other.day_number
            || (self.day_number == other.day_number
                && self.seconds_of_day > other.seconds_of_day)
    }

    /// 若两个日期相差在 epsilon 秒以内则返回 true。
    /// 映射到 `JulianDate.equalsEpsilon`
    pub fn equals_epsilon(&self, other: &Self, epsilon: f64) -> bool {
        self.seconds_difference(other).abs() <= epsilon
    }

    /// 转换为 GregorianDate。
    /// 映射到 `JulianDate.toGregorianDate`
    ///
    /// 内部将 TAI→UTC。若处于闰秒期间，则标记 `is_leap_second = true`
    /// 并使用 second=60。
    pub fn to_gregorian_date(&self) -> GregorianDate {
        let mut is_leap_second = false;

        // 将 TAI 转换为 UTC
        let (utc_day_number, utc_seconds_of_day) =
            match convert_tai_to_utc(self.day_number, self.seconds_of_day) {
                Some(v) => v,
                None => {
                    // 处于闰秒期间：减去 1 秒并再次转换
                    let adjusted = self.add_seconds(-1.0);
                    is_leap_second = true;
                    convert_tai_to_utc(adjusted.day_number, adjusted.seconds_of_day)
                        .unwrap_or((adjusted.day_number, adjusted.seconds_of_day))
                }
            };

        let mut julian_day_number = utc_day_number;
        let seconds_of_day = utc_seconds_of_day;

        if seconds_of_day >= 43200.0 {
            julian_day_number += 1;
        }

        // 算法取自《Explanatory Supplement to the
        // Astronomical Almanac》（Seidelmann 1992）第 604 页。
        let l = julian_day_number + 68569;
        let n = (4 * l) / 146097;
        let l = l - (146097 * n + 3) / 4;
        let i = (4000 * (l + 1)) / 1461001;
        let l = l - (1461 * i) / 4 + 31;
        let j = (80 * l) / 2447;
        let day = (l - (2447 * j) / 80) as u32;
        let l = j / 11;
        let month = (j + 2 - 12 * l) as u32;
        let year = (100 * (n - 49) + i + l) as i32;

        let mut hour = (seconds_of_day / SECONDS_PER_HOUR) as u32;
        let mut remaining_seconds = seconds_of_day - hour as f64 * SECONDS_PER_HOUR;
        let minute = (remaining_seconds / SECONDS_PER_MINUTE) as u32;
        remaining_seconds -= minute as f64 * SECONDS_PER_MINUTE;
        let mut second = remaining_seconds as u32;
        let millisecond = (remaining_seconds - second as f64) / SECONDS_PER_MILLISECOND;

        // JulianDate 以正午为基准
        hour += 12;
        if hour > 23 {
            hour -= 24;
        }

        // 若我们处于闰秒，则将其加回（second 变为 60）
        if is_leap_second {
            second += 1;
        }

        GregorianDate::new(year, month, day, hour, minute, second, millisecond, is_leap_second)
    }

    /// 转换为 ISO 8601 字符串表示。
    /// 映射到 `JulianDate.toIso8601`
    ///
    /// 若 `precision` 为 None，使用最高精度的表示（去除尾随零）。
    /// 若 `precision` 为 Some(n)，小数秒恰好格式化为 n 位。
    pub fn to_iso8601(&self) -> String {
        self.to_iso8601_with_precision(None)
    }

    /// 以指定精度转换为 ISO 8601 字符串。
    pub fn to_iso8601_with_precision(&self, precision: Option<usize>) -> String {
        let g = self.to_gregorian_date();
        let mut year = g.year;
        let mut month = g.month;
        let mut day = g.day;
        let mut hour = g.hour;
        let minute = g.minute;
        let second = g.second;
        let millisecond = g.millisecond;

        // 特殊情况：Iso8601.MAXIMUM_VALUE (10000-01-01T00:00:00 = 9999-12-31T24:00:00)
        if year == 10000 && month == 1 && day == 1 && hour == 0 && minute == 0 && second == 0
            && millisecond.abs() < 1e-10
        {
            year = 9999;
            month = 12;
            day = 31;
            hour = 24;
        }

        let base = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            year, month, day, hour, minute, second
        );

        match precision {
            Some(0) => format!("{}Z", base),
            Some(p) => {
                // 复刻 CesiumJS：(millisecond * 0.01).toFixed(precision).replace(".","").slice(0,precision)
                let frac = millisecond * 0.01;
                let s = format!("{:.prec$}", frac, prec = p);
                // p=3 时 s = "0.050" → 去掉 '.' → "0050" → 取前 p 个字符 → "005"
                let no_dot: String = s.chars().filter(|&c| c != '.').collect();
                let result = &no_dot[..p.min(no_dot.len())];
                format!("{}.{}Z", base, result)
            }
            None => {
                // 最高精度表示：去除尾随零
                if millisecond.abs() < 1e-10 {
                    format!("{}Z", base)
                } else {
                    let frac_seconds = millisecond / 1000.0;
                    // 使用足够的精度以捕获该值
                    let frac_str = format!("{:.15}", frac_seconds);
                    // 去除尾随零和前导的 "0"
                    let trimmed = frac_str.trim_end_matches('0');
                    let digits = &trimmed[1..]; // 去除前导 "0"
                    if digits == "." || digits.is_empty() {
                        format!("{}Z", base)
                    } else {
                        format!("{}{}Z", base, digits)
                    }
                }
            }
        }
    }

    /// Unix 纪元（1970-01-01T00:00:00Z）的儒略日。
    pub const UNIX_EPOCH: Self = Self {
        day_number: 2440588,
        seconds_of_day: 0.0, // 以正午为基准，因此午夜 = 正午前 43200 秒
    };

    /// J2000 纪元（2000-01-01T12:00:00 TAI）的儒略日。
    pub const J2000: Self = Self {
        day_number: 2451545,
        seconds_of_day: 0.0,
    };
}

impl PartialOrd for JulianDate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for JulianDate {}

impl Ord for JulianDate {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.day_number.cmp(&other.day_number) {
            Ordering::Equal => self.seconds_of_day.partial_cmp(&other.seconds_of_day).unwrap_or(Ordering::Equal),
            ord => ord,
        }
    }
}

impl Default for JulianDate {
    fn default() -> Self {
        Self {
            day_number: 0,
            seconds_of_day: 0.0,
        }
    }
}

/// 从格里高利日期分量计算儒略日分量。
/// 算法取自《Explanatory Supplement to the
/// Astronomical Almanac》（Seidelmann 1992）第 604 页。
/// 映射到 `computeJulianDateComponents`
fn compute_julian_date_components(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    millisecond: f64,
) -> (i64, f64) {
    let a = (month as i64 - 14) / 12;
    let b = year as i64 + 4800 + a;
    let mut day_number = (1461 * b / 4)
        + (367 * (month as i64 - 2 - 12 * a) / 12)
        - (3 * ((b + 100) / 100) / 4)
        + day as i64
        - 32075;

    // JulianDate 以正午为基准
    let mut hour = hour as f64 - 12.0;
    if hour < 0.0 {
        hour += 24.0;
    }

    let seconds_of_day = second as f64
        + hour * SECONDS_PER_HOUR
        + minute as f64 * SECONDS_PER_MINUTE
        + millisecond * SECONDS_PER_MILLISECOND;

    if seconds_of_day >= 43200.0 {
        day_number -= 1;
    }

    (day_number, seconds_of_day)
}

// ============================================================================
// ISO 8601 解析辅助函数
// ============================================================================

/// 解析 YYYY-MM-DD 或 YYYYMMDD
fn parse_calendar_date(s: &str) -> Option<(i32, i32, i32)> {
    let bytes = s.as_bytes();
    if bytes.len() == 10 && bytes[4] == b'-' && bytes[7] == b'-' {
        // YYYY-MM-DD
        let y: i32 = s[0..4].parse().ok()?;
        let m: i32 = s[5..7].parse().ok()?;
        let d: i32 = s[8..10].parse().ok()?;
        Some((y, m, d))
    } else if bytes.len() == 8 && bytes.iter().all(|b| b.is_ascii_digit()) {
        // YYYYMMDD
        let y: i32 = s[0..4].parse().ok()?;
        let m: i32 = s[4..6].parse().ok()?;
        let d: i32 = s[6..8].parse().ok()?;
        Some((y, m, d))
    } else {
        None
    }
}

/// 解析 YYYY-MM（第 4 位恰好一个短横线）
fn parse_calendar_month(s: &str) -> Option<(i32, i32)> {
    let bytes = s.as_bytes();
    if bytes.len() == 7 && bytes[4] == b'-' {
        let y: i32 = s[0..4].parse().ok()?;
        let m: i32 = s[5..7].parse().ok()?;
        Some((y, m))
    } else {
        None
    }
}

/// 解析 YYYY（恰好 4 位数字）
fn parse_calendar_year(s: &str) -> Option<i32> {
    if s.len() == 4 && s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse().ok()
    } else {
        None
    }
}

/// 解析 YYYY-DDD 或 YYYYDDD（序数日期）
fn parse_ordinal_date(s: &str) -> Option<(i32, i32)> {
    let bytes = s.as_bytes();
    if bytes.len() == 8 && bytes[4] == b'-' {
        // YYYY-DDD
        let y: i32 = s[0..4].parse().ok()?;
        let doy: i32 = s[5..8].parse().ok()?;
        Some((y, doy))
    } else if bytes.len() == 7 && bytes.iter().all(|b| b.is_ascii_digit()) {
        // YYYYDDD
        let y: i32 = s[0..4].parse().ok()?;
        let doy: i32 = s[4..7].parse().ok()?;
        Some((y, doy))
    } else {
        None
    }
}

/// 解析 YYYY-Www-D、YYYYWwwD、YYYY-Www 或 YYYYWww（周日期）
fn parse_week_date(s: &str) -> Option<(i32, i32, i32)> {
    let bytes = s.as_bytes();
    if bytes.len() == 8 && bytes[4] == b'-' && bytes[5] == b'W' && bytes[6].is_ascii_digit() && bytes[7].is_ascii_digit() {
        // YYYY-Www（无日）
        let y: i32 = s[0..4].parse().ok()?;
        let w: i32 = s[6..8].parse().ok()?;
        Some((y, w, 0))
    } else if bytes.len() == 10 && bytes[4] == b'-' && bytes[5] == b'W' && bytes[8] == b'-' {
        // YYYY-Www-D
        let y: i32 = s[0..4].parse().ok()?;
        let w: i32 = s[6..8].parse().ok()?;
        let d: i32 = s[9..10].parse().ok()?;
        Some((y, w, d))
    } else if bytes.len() == 7 && bytes[4] == b'W' && bytes[5].is_ascii_digit() && bytes[6].is_ascii_digit() {
        // YYYYWww
        let y: i32 = s[0..4].parse().ok()?;
        let w: i32 = s[5..7].parse().ok()?;
        Some((y, w, 0))
    } else if bytes.len() == 8 && bytes[4] == b'W' && bytes.iter().skip(5).all(|b| b.is_ascii_digit()) {
        // YYYYWwwD
        let y: i32 = s[0..4].parse().ok()?;
        let w: i32 = s[5..7].parse().ok()?;
        let d: i32 = s[7..8].parse().ok()?;
        Some((y, w, d))
    } else {
        None
    }
}

/// 将序数日（一年中的第几天）转换为 (month, day)
fn ordinal_to_month_day(year: i32, day_of_year: i32) -> (i32, i32) {
    let days_in_month_arr: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut remaining = day_of_year;
    // deferred.md #13: 以 m 索引 days_in_month_arr，clippy 建议迭代器；保持索引写法可读性。
    #[allow(clippy::needless_range_loop)]
    for m in 0..12 {
        let dim = if m == 1 && crate::gregorian_date::is_leap_year(year) {
            29
        } else {
            days_in_month_arr[m]
        };
        if remaining <= dim {
            return (m as i32 + 1, remaining);
        }
        remaining -= dim;
    }
    (12, remaining)
}

/// 计算给定日期的星期。返回 0=周日、1=周一、……、6=周六。
/// 使用儒略日数算法。
fn day_of_week(year: i32, month: i32, day: i32) -> i32 {
    let (dn, _) = compute_julian_date_components(year, month as u32, day as u32, 12, 0, 0, 0.0);
    // 儒略日 0 是周一。(dn + 1) % 7 得到 0=周日。
    ((dn + 1) % 7) as i32
}

/// 解析时间字符串：HH、HH:MM、HH:MM:SS，可带小数部分和 UTC 偏移。
/// 返回 (hour, minute, second, millisecond, offset_char, offset_hours, offset_minutes)
/// offset_char：'Z'、'+'、'-' 或 '\0'（无）
#[allow(clippy::type_complexity)]
fn parse_time(s: &str) -> Option<(i32, i32, f64, f64, char, i32, i32)> {
    // 先尝试 HH:MM:SS 或 HHMMSS（最具体）
    if let Some(result) = parse_hms(s) {
        return Some(result);
    }
    // 尝试 HH:MM 或 HHMM
    if let Some(result) = parse_hm(s) {
        return Some(result);
    }
    // 尝试 HH
    parse_h(s)
}

/// 解析 HH:MM:SS.fraction，可带 UTC 偏移
fn parse_hms(s: &str) -> Option<(i32, i32, f64, f64, char, i32, i32)> {
    // 找到时间数字结束、偏移开始的位置
    let (time_part, offset_char, offset_hours, offset_minutes) = split_offset(s)?;

    // time_part 应为 HH:MM:SS、HHMMSS、HH:MM:SS.fraction、HHMMSS.fraction
    let bytes = time_part.as_bytes();
    if bytes.len() < 6 {
        return None;
    }

    let (h_str, m_str, s_str) = if bytes.len() >= 5 && bytes[2] == b':' {
        // 扩展格式：HH:MM:SS...
        if bytes.len() < 8 || bytes[5] != b':' {
            return None;
        }
        let rest = &time_part[6..];
        // rest 为 SS 或 SS.fraction
        let dot_pos = rest.find('.');
        let (sec_str, _frac_str) = match dot_pos {
            Some(p) => (&rest[..p], &rest[p..]),
            None => (rest, ""),
        };
        if sec_str.len() != 2 {
            return None;
        }
        (&time_part[0..2], &time_part[3..5], rest)
    } else {
        // 基本格式：HHMMSS...
        let rest = &time_part[4..]; // SS 或 SS.fraction
        // 若 rest 以 '.' 开头，则为 HHMM.fraction（小数分钟），而非 HHMMSS
        if rest.is_empty() || rest.as_bytes()[0] == b'.' {
            return None;
        }
        (&time_part[0..2], &time_part[2..4], rest)
    };

    let hour: i32 = h_str.parse().ok()?;
    let minute: i32 = m_str.parse().ok()?;

    // 解析秒，可带小数部分
    let second: f64 = s_str.parse().ok()?;
    let sec_int = second as i32;
    let millisecond = (second - sec_int as f64) * 1000.0;

    Some((hour, minute, sec_int as f64, millisecond, offset_char, offset_hours, offset_minutes))
}

/// 解析 HH:MM 或 HHMM，可带小数部分和 UTC 偏移
fn parse_hm(s: &str) -> Option<(i32, i32, f64, f64, char, i32, i32)> {
    let (time_part, offset_char, offset_hours, offset_minutes) = split_offset(s)?;

    let bytes = time_part.as_bytes();
    if bytes.len() < 4 {
        return None;
    }

    let (h_str, m_str) = if bytes.len() >= 5 && bytes[2] == b':' {
        // 扩展格式：HH:MM 或 HH:MM.fraction
        (&time_part[0..2], &time_part[3..])
    } else if bytes.len() >= 4 && bytes[0].is_ascii_digit() && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit() && bytes[3].is_ascii_digit()
    {
        // 基本格式：HHMM 或 HHMM.fraction
        (&time_part[0..2], &time_part[2..])
    } else {
        return None;
    };

    let hour: i32 = h_str.parse().ok()?;
    // m_str 可能带小数部分
    let minute_val: f64 = m_str.parse().ok()?;
    let minute_int = minute_val as i32;
    let second = (minute_val - minute_int as f64) * 60.0;

    Some((hour, minute_int, second, 0.0, offset_char, offset_hours, offset_minutes))
}

/// 解析 HH，可带小数部分和 UTC 偏移
fn parse_h(s: &str) -> Option<(i32, i32, f64, f64, char, i32, i32)> {
    let (time_part, offset_char, offset_hours, offset_minutes) = split_offset(s)?;

    if time_part.len() < 2 {
        return None;
    }

    let hour: i32 = time_part[0..2].parse().ok()?;
    // 可选的小数小时
    let minute = if time_part.len() > 2 {
        let frac: f64 = time_part[2..].parse().ok()?;
        frac * 60.0
    } else {
        0.0
    };

    Some((hour, minute as i32, (minute - minute as i32 as f64) * 60.0, 0.0, offset_char, offset_hours, offset_minutes))
}

/// 将时间字符串拆分为 (time_digits, offset_char, offset_hours, offset_minutes)。
/// 处理：Z、+HH、+HH:MM、-HH、-HH:MM 或无偏移。
fn split_offset(s: &str) -> Option<(&str, char, i32, i32)> {
    // 查找表示偏移的 Z、+ 或 -
    // 偏移只能出现在时间数字之后
    if let Some(z_pos) = s.find('Z') {
        if z_pos == s.len() - 1 {
            return Some((&s[..z_pos], 'Z', 0, 0));
        }
        return None;
    }

    // 查找 + 或 -（但不在位置 0）
    for (i, c) in s.char_indices() {
        if (c == '+' || c == '-') && i > 0 {
            let time_part = &s[..i];
            let offset_str = &s[i + 1..];
            let (oh, om) = parse_offset_value(offset_str)?;
            return Some((time_part, c, oh, om));
        }
    }

    // 无偏移
    Some((s, '\0', 0, 0))
}

/// 解析偏移值：HH、HH:MM 或 HHMM
fn parse_offset_value(s: &str) -> Option<(i32, i32)> {
    if s.is_empty() {
        return Some((0, 0));
    }
    if s.len() == 2 {
        let h: i32 = s.parse().ok()?;
        Some((h, 0))
    } else if s.len() == 4 {
        let h: i32 = s[0..2].parse().ok()?;
        let m: i32 = s[2..4].parse().ok()?;
        Some((h, m))
    } else if s.len() == 5 && s.as_bytes()[2] == b':' {
        let h: i32 = s[0..2].parse().ok()?;
        let m: i32 = s[3..5].parse().ok()?;
        Some((h, m))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_date_components_j2000() {
        // J2000 纪元：2000-01-01T12:00:00 UTC
        // 内部以 TAI 存储（在该日期为 UTC + 32s）
        let jd = JulianDate::from_date_components(2000, 1, 1, 12, 0, 0, 0.0);
        // 通过往返转换校验
        let g = jd.to_gregorian_date();
        assert_eq!(g.year, 2000);
        assert_eq!(g.month, 1);
        assert_eq!(g.day, 1);
        assert_eq!(g.hour, 12);
        assert_eq!(g.minute, 0);
        assert_eq!(g.second, 0);
    }

    #[test]
    fn test_from_date_components_unix_epoch() {
        // Unix 纪元：1970-01-01T00:00:00 UTC
        let jd = JulianDate::from_date_components(1970, 1, 1, 0, 0, 0, 0.0);
        // 通过往返转换校验
        let g = jd.to_gregorian_date();
        assert_eq!(g.year, 1970);
        assert_eq!(g.month, 1);
        assert_eq!(g.day, 1);
        assert_eq!(g.hour, 0);
        assert_eq!(g.minute, 0);
        assert_eq!(g.second, 0);
    }

    #[test]
    fn test_seconds_difference() {
        let jd1 = JulianDate::from_date_components(2000, 1, 1, 12, 0, 0, 0.0);
        let jd2 = JulianDate::from_date_components(2000, 1, 1, 12, 0, 1, 0.0);
        assert!((jd2.seconds_difference(&jd1) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_days_difference() {
        let jd1 = JulianDate::from_date_components(2000, 1, 1, 0, 0, 0, 0.0);
        let jd2 = JulianDate::from_date_components(2000, 1, 2, 0, 0, 0, 0.0);
        assert!((jd2.days_difference(&jd1) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_seconds() {
        let jd = JulianDate::from_date_components(2000, 1, 1, 12, 0, 0, 0.0);
        let jd2 = jd.add_seconds(3600.0);
        let g = jd2.to_gregorian_date();
        assert_eq!(g.hour, 13);
        assert_eq!(g.minute, 0);
    }

    #[test]
    fn test_add_days() {
        let jd = JulianDate::from_date_components(2000, 1, 1, 12, 0, 0, 0.0);
        let jd2 = jd.add_days(1.0);
        let g = jd2.to_gregorian_date();
        assert_eq!(g.day, 2);
    }

    #[test]
    fn test_to_gregorian_roundtrip() {
        let jd = JulianDate::from_date_components(2023, 6, 15, 14, 30, 45, 500.0);
        let g = jd.to_gregorian_date();
        assert_eq!(g.year, 2023);
        assert_eq!(g.month, 6);
        assert_eq!(g.day, 15);
        assert_eq!(g.hour, 14);
        assert_eq!(g.minute, 30);
        assert_eq!(g.second, 45);
        assert!((g.millisecond - 500.0).abs() < 1.0);
    }

    #[test]
    fn test_comparison() {
        let jd1 = JulianDate::from_date_components(2000, 1, 1, 0, 0, 0, 0.0);
        let jd2 = JulianDate::from_date_components(2000, 1, 2, 0, 0, 0, 0.0);
        assert!(jd1.less_than(&jd2));
        assert!(jd2.greater_than(&jd1));
        assert!(!jd1.equals_epsilon(&jd2, 0.001));
        assert!(jd1.equals_epsilon(&jd2, 86401.0));
    }

    #[test]
    fn test_total_days() {
        // J2000 基于 TAI 的 total_days 包含 32s 闰偏移
        let jd = JulianDate::from_date_components(2000, 1, 1, 12, 0, 0, 0.0);
        // total_days 为 TAI：2451545 + 32/86400
        let expected = 2451545.0 + 32.0 / 86400.0;
        assert!((jd.total_days() - expected).abs() < 1e-10);
    }

    #[test]
    fn test_unix_seconds_roundtrip() {
        let jd = JulianDate::from_date_components(2020, 6, 15, 12, 0, 0, 0.0);
        let unix = jd.to_unix_seconds();
        let jd2 = JulianDate::from_unix_seconds(unix);
        assert!(jd.equals_epsilon(&jd2, 0.001));
    }

    #[test]
    fn test_from_iso8601_basic() {
        let jd = JulianDate::from_iso8601("2008-11-12T05:30:00Z").unwrap();
        let g = jd.to_gregorian_date();
        assert_eq!(g.year, 2008);
        assert_eq!(g.month, 11);
        assert_eq!(g.day, 12);
        assert_eq!(g.hour, 5);
        assert_eq!(g.minute, 30);
        assert_eq!(g.second, 0);
    }

    #[test]
    fn test_from_iso8601_invalid() {
        assert!(JulianDate::from_iso8601("").is_none());
        assert!(JulianDate::from_iso8601("foobar").is_none());
        assert!(JulianDate::from_iso8601("2008-13-01T00:00:00Z").is_none());
    }
}
