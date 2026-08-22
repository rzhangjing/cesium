//! Ported from packages/engine/Source/Core/JulianDate.js

use regex::Regex;
use std::sync::LazyLock;

use crate::binary_search::binary_search;
use crate::gregorian_date::GregorianDate;
use crate::is_leap_year::is_leap_year;
use crate::leap_second::LeapSecond;
use crate::time_constants::*;
use crate::time_standard::TimeStandard;

const DAYS_IN_MONTH: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
const DAYS_IN_LEAP_FEBRUARY: i32 = 29;

/// Represents an astronomical Julian date — the number of days since noon on
/// January 1, −4712 (4713 BC).
///
/// For increased precision, this class stores the whole number part of the
/// date and the seconds part of the date in separate components.  The date is
/// always stored in the International Atomic Time (TAI) standard.
#[derive(Debug, Clone)]
pub struct JulianDate {
    /// The number of whole days.
    pub day_number: i32,
    /// The number of seconds into the current day.
    pub seconds_of_day: f64,
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn set_components(whole_days: i32, seconds_of_day: f64, julian_date: &mut JulianDate) {
    let extra_days = (seconds_of_day / SECONDS_PER_DAY) as i32;
    let mut whole_days = whole_days + extra_days;
    let mut seconds_of_day = seconds_of_day - SECONDS_PER_DAY * extra_days as f64;

    if seconds_of_day < 0.0 {
        whole_days -= 1;
        seconds_of_day += SECONDS_PER_DAY;
    }

    julian_date.day_number = whole_days;
    julian_date.seconds_of_day = seconds_of_day;
}

fn compute_julian_date_components(
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: i32,
    millisecond: f64,
) -> (i32, f64) {
    // Algorithm from page 604 of the Explanatory Supplement to the
    // Astronomical Almanac (Seidelmann 1992).
    let a = (month - 14) / 12;
    let b = year + 4800 + a;
    let mut day_number = (1461 * b) / 4
        + (367 * (month - 2 - 12 * a)) / 12
        - (3 * ((b + 100) / 100)) / 4
        + day
        - 32075;

    // JulianDates are noon-based
    let mut hour = hour - 12;
    if hour < 0 {
        hour += 24;
    }

    let seconds_of_day = second as f64
        + (hour as f64 * SECONDS_PER_HOUR
            + minute as f64 * SECONDS_PER_MINUTE
            + millisecond * SECONDS_PER_MILLISECOND);

    if seconds_of_day >= 43200.0 {
        day_number -= 1;
    }

    (day_number, seconds_of_day)
}

fn compare_leap_second_dates(leap_second: &LeapSecond, date_to_find: &JulianDate) -> f64 {
    JulianDate::compare(&leap_second.julian_date, date_to_find) as f64
}

fn convert_utc_to_tai(julian_date: &mut JulianDate) {
    let leap_seconds = leap_seconds_table();
    let mut index = binary_search(leap_seconds, julian_date, |ls, jd| {
        compare_leap_second_dates(ls, jd)
    });

    if index < 0 {
        index = !index;
    }

    if index >= leap_seconds.len() as i64 {
        index = leap_seconds.len() as i64 - 1;
    }

    let mut offset = leap_seconds[index as usize].offset;
    if index > 0 {
        let difference =
            JulianDate::seconds_difference(&leap_seconds[index as usize].julian_date, julian_date);
        if difference > offset {
            index -= 1;
            offset = leap_seconds[index as usize].offset;
        }
    }

    JulianDate::add_seconds_mut(julian_date, offset);
}

/// Returns `None` when the instant falls on a leap second (cannot represent in UTC).
fn convert_tai_to_utc(julian_date: &JulianDate, result: &mut JulianDate) -> Option<()> {
    let leap_seconds = leap_seconds_table();
    let mut index = binary_search(leap_seconds, julian_date, |ls, jd| {
        compare_leap_second_dates(ls, jd)
    });
    if index < 0 {
        index = !index;
    }

    if index == 0 {
        *result = JulianDate::add_seconds_new(julian_date, -leap_seconds[0].offset);
        return Some(());
    }

    if index >= leap_seconds.len() as i64 {
        *result =
            JulianDate::add_seconds_new(julian_date, -leap_seconds[index as usize - 1].offset);
        return Some(());
    }

    let difference = JulianDate::seconds_difference(
        &leap_seconds[index as usize].julian_date,
        julian_date,
    );

    if difference == 0.0 {
        *result =
            JulianDate::add_seconds_new(julian_date, -leap_seconds[index as usize].offset);
        return Some(());
    }

    if difference <= 1.0 {
        // During a leap second – cannot convert to UTC
        return None;
    }

    *result =
        JulianDate::add_seconds_new(julian_date, -leap_seconds[index as usize - 1].offset);
    Some(())
}

fn leap_seconds_table() -> &'static [LeapSecond] {
    static TABLE: LazyLock<Vec<LeapSecond>> = LazyLock::new(|| {
        vec![
            LeapSecond::new(JulianDate::from_tai_components(2441317, 43210.0), 10.0),
            LeapSecond::new(JulianDate::from_tai_components(2441499, 43211.0), 11.0),
            LeapSecond::new(JulianDate::from_tai_components(2441683, 43212.0), 12.0),
            LeapSecond::new(JulianDate::from_tai_components(2442048, 43213.0), 13.0),
            LeapSecond::new(JulianDate::from_tai_components(2442413, 43214.0), 14.0),
            LeapSecond::new(JulianDate::from_tai_components(2442778, 43215.0), 15.0),
            LeapSecond::new(JulianDate::from_tai_components(2443144, 43216.0), 16.0),
            LeapSecond::new(JulianDate::from_tai_components(2443509, 43217.0), 17.0),
            LeapSecond::new(JulianDate::from_tai_components(2443874, 43218.0), 18.0),
            LeapSecond::new(JulianDate::from_tai_components(2444239, 43219.0), 19.0),
            LeapSecond::new(JulianDate::from_tai_components(2444786, 43220.0), 20.0),
            LeapSecond::new(JulianDate::from_tai_components(2445151, 43221.0), 21.0),
            LeapSecond::new(JulianDate::from_tai_components(2445516, 43222.0), 22.0),
            LeapSecond::new(JulianDate::from_tai_components(2446247, 43223.0), 23.0),
            LeapSecond::new(JulianDate::from_tai_components(2447161, 43224.0), 24.0),
            LeapSecond::new(JulianDate::from_tai_components(2447892, 43225.0), 25.0),
            LeapSecond::new(JulianDate::from_tai_components(2448257, 43226.0), 26.0),
            LeapSecond::new(JulianDate::from_tai_components(2448804, 43227.0), 27.0),
            LeapSecond::new(JulianDate::from_tai_components(2449169, 43228.0), 28.0),
            LeapSecond::new(JulianDate::from_tai_components(2449534, 43229.0), 29.0),
            LeapSecond::new(JulianDate::from_tai_components(2450083, 43230.0), 30.0),
            LeapSecond::new(JulianDate::from_tai_components(2450630, 43231.0), 31.0),
            LeapSecond::new(JulianDate::from_tai_components(2451179, 43232.0), 32.0),
            LeapSecond::new(JulianDate::from_tai_components(2453736, 43233.0), 33.0),
            LeapSecond::new(JulianDate::from_tai_components(2454832, 43234.0), 34.0),
            LeapSecond::new(JulianDate::from_tai_components(2456109, 43235.0), 35.0),
            LeapSecond::new(JulianDate::from_tai_components(2457204, 43236.0), 36.0),
            LeapSecond::new(JulianDate::from_tai_components(2457754, 43237.0), 37.0),
        ]
    });
    &TABLE
}

// ── ISO 8601 regex patterns ─────────────────────────────────────────────────

static MATCH_CALENDAR_YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{4})$").unwrap());
static MATCH_CALENDAR_MONTH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{4})-(\d{2})$").unwrap());
static MATCH_ORDINAL_DATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{4})-?(\d{3})$").unwrap());
static MATCH_WEEK_DATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{4})-?W(\d{2})-?(\d{1})?$").unwrap());
static MATCH_CALENDAR_DATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{4})-?(\d{2})-?(\d{2})$").unwrap());

// Time patterns – each ends with a UTC-offset capture group
static UTC_OFFSET: &str = r"([Z+\-])?(\d{2})?:?(\d{2})?$";
static MATCH_HMS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^(\d{{2}}):?(\d{{2}}):?(\d{{2}})(?:[.,](\d+))?{}",
        UTC_OFFSET
    ))
    .unwrap()
});
static MATCH_HM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^(\d{{2}}):?(\d{{2}})(?:[.,](\d+))?{}",
        UTC_OFFSET
    ))
    .unwrap()
});
static MATCH_H: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"^(\d{{2}})(?:[.,](\d+))?{}", UTC_OFFSET)).unwrap()
});

// ── Implementation ───────────────────────────────────────────────────────────

impl JulianDate {
    // ── constructors ─────────────────────────────────────────────────────

    /// Creates a new `JulianDate` from day number, seconds-of-day and a time
    /// standard.  Defaults: `day_number = 0`, `seconds_of_day = 0.0`,
    /// `time_standard = UTC`.
    pub fn new(
        julian_day_number: f64,
        seconds_of_day: f64,
        time_standard: TimeStandard,
    ) -> Self {
        let whole_days = julian_day_number as i32;
        let seconds_of_day =
            seconds_of_day + (julian_day_number - whole_days as f64) * SECONDS_PER_DAY;

        let mut result = JulianDate {
            day_number: 0,
            seconds_of_day: 0.0,
        };
        set_components(whole_days, seconds_of_day, &mut result);

        if time_standard == TimeStandard::UTC {
            convert_utc_to_tai(&mut result);
        }
        result
    }

    /// Creates a default `JulianDate` (equivalent to `new JulianDate()` in JS).
    pub fn default_date() -> Self {
        Self::new(0.0, 0.0, TimeStandard::UTC)
    }

    /// Creates a `JulianDate` representing the current system time.
    /// Port of CesiumJS `JulianDate.now()`.
    pub fn now() -> Self {
        use std::time::SystemTime;
        let duration = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let millis = duration.as_secs_f64() * 1000.0;
        // Julian day 0 = -4712-01-01T12:00:00Z
        // Unix epoch (1970-01-01T00:00:00Z) = Julian day 2440587.5
        let julian_day = 2440587.5 + millis / 86400000.0;
        let whole_days = julian_day.floor();
        let seconds_of_day = (julian_day - whole_days) * SECONDS_PER_DAY;
        Self::new(whole_days, seconds_of_day, TimeStandard::UTC)
    }

    /// Internal helper – build a TAI date directly from integer components
    /// without going through the constructor's UTC conversion.
    pub fn from_tai_components(day_number: i32, seconds_of_day: f64) -> Self {
        let mut result = JulianDate {
            day_number: 0,
            seconds_of_day: 0.0,
        };
        set_components(day_number, seconds_of_day, &mut result);
        result
    }

    // ── from_gregorian_date ──────────────────────────────────────────────

    /// Creates a `JulianDate` from a [`GregorianDate`].
    pub fn from_gregorian_date(date: &GregorianDate) -> Self {
        let (dn, sod) = compute_julian_date_components(
            date.year,
            date.month,
            date.day,
            date.hour,
            date.minute,
            date.second,
            date.millisecond,
        );
        let mut result = Self::new(dn as f64, sod, TimeStandard::UTC);
        if date.is_leap_second {
            result = Self::add_seconds_new(&result, 1.0);
        }
        result
    }

    // ── from_date (chrono-free: we accept y/m/d/h/m/s/ms) ────────────────

    /// Creates a `JulianDate` from date-time components (UTC).
    ///
    /// This is the Rust equivalent of `JulianDate.fromDate(new Date(...))`.
    pub fn from_date_components(
        year: i32,
        month: i32,
        day: i32,
        hour: i32,
        minute: i32,
        second: i32,
        millisecond: f64,
    ) -> Self {
        let (dn, sod) =
            compute_julian_date_components(year, month, day, hour, minute, second, millisecond);
        Self::new(dn as f64, sod, TimeStandard::UTC)
    }

    // ── from_iso8601 ─────────────────────────────────────────────────────

    /// Parses an ISO 8601 date-time string and returns a `JulianDate`.
    pub fn from_iso8601(iso8601: &str) -> Option<Self> {
        // Split on 'T' to separate date and time
        let parts: Vec<&str> = iso8601.split('T').collect();
        if parts.len() > 2 {
            return None; // interval or invalid
        }

        let date_str = parts[0];
        let time_str = if parts.len() == 2 { Some(parts[1]) } else { None };

        let mut year: i32;
        let mut month: i32 = 1;
        let mut day: i32 = 1;
        let mut day_of_year: Option<i32> = None;
        let mut week_number: Option<i32> = None;
        let mut day_of_week: Option<i32> = None;

        // ── Parse date ───────────────────────────────────────────────────
        if let Some(caps) = MATCH_CALENDAR_DATE.captures(date_str) {
            year = caps[1].parse().ok()?;
            month = caps[2].parse().ok()?;
            day = caps[3].parse().ok()?;

            // Validate basic vs extended format consistency
            let dash_count = date_str.matches('-').count();
            if dash_count > 0 && dash_count != 2 {
                return None;
            }
        } else if let Some(caps) = MATCH_CALENDAR_MONTH.captures(date_str) {
            year = caps[1].parse().ok()?;
            month = caps[2].parse().ok()?;
            day = 1;
        } else if let Some(caps) = MATCH_CALENDAR_YEAR.captures(date_str) {
            year = caps[1].parse().ok()?;
            month = 1;
            day = 1;
        } else if let Some(caps) = MATCH_ORDINAL_DATE.captures(date_str) {
            year = caps[1].parse().ok()?;
            day_of_year = Some(caps[2].parse().ok()?);
            let dash_count = date_str.matches('-').count();
            if dash_count > 1 {
                return None;
            }
        } else if let Some(caps) = MATCH_WEEK_DATE.captures(date_str) {
            year = caps[1].parse().ok()?;
            week_number = Some(caps[2].parse().ok()?);
            day_of_week = caps.get(3).map(|m| m.as_str().parse().ok()).flatten();
            if day_of_week.is_none() {
                day_of_week = Some(1);
            }
            let dash_count = date_str.matches('-').count();
            if dash_count > 0 && dash_count != 2 {
                return None;
            }
        } else {
            return None;
        }

        // Resolve ordinal / week dates to month/day
        if let Some(doy) = day_of_year {
            let in_leap = is_leap_year(year as f64);
            let max_day = if in_leap { 366 } else { 365 };
            if doy < 1 || doy > max_day {
                return None;
            }
            // Convert day-of-year to month/day
            let mut remaining = doy;
            let feb_days = if in_leap { 29 } else { 28 };
            let month_lengths = [31, feb_days, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
            month = 1;
            for &ml in &month_lengths {
                if remaining <= ml {
                    day = remaining;
                    break;
                }
                remaining -= ml;
                month += 1;
            }
            if remaining == 0 && month > 12 {
                return None;
            }
        } else if let Some(wn) = week_number {
            // ISO week date → ordinal day
            let jan4 = compute_julian_date_components(year, 1, 4, 0, 0, 0, 0.0);
            // Find day-of-week of Jan 4 (0=Sun .. 6=Sat)
            let jan4_jd = jan4.0 as f64 + (jan4.1 + 43200.0) / SECONDS_PER_DAY;
            let dow_jan4 = ((jan4_jd + 1.5) % 7.0) as i32; // 0=Sun
            let doy = wn * 7 + day_of_week.unwrap_or(1) - dow_jan4 - 3;
            if doy < 1 {
                return None;
            }
            let in_leap = is_leap_year(year as f64);
            let max_day = if in_leap { 366 } else { 365 };
            if doy > max_day {
                return None;
            }
            let mut remaining = doy;
            let feb_days = if in_leap { 29 } else { 28 };
            let month_lengths = [31, feb_days, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
            month = 1;
            for &ml in &month_lengths {
                if remaining <= ml {
                    day = remaining;
                    break;
                }
                remaining -= ml;
                month += 1;
            }
        }

        // Validate date ranges
        let in_leap_year = is_leap_year(year as f64);
        if month < 1 || month > 12 || day < 1 {
            return None;
        }
        let max_day = if in_leap_year && month == 2 {
            DAYS_IN_LEAP_FEBRUARY
        } else {
            DAYS_IN_MONTH[(month - 1) as usize]
        };
        if day > max_day {
            return None;
        }

        // ── Parse time ───────────────────────────────────────────────────
        let mut hour: i32 = 0;
        let mut minute: i32 = 0;
        let mut second: i32 = 0;
        let mut millisecond: f64 = 0.0;

        // Deferred UTC offset: (sign, offset_hours, offset_minutes)

        if let Some(ts) = time_str {
            // utc_offset initial None is always overwritten before read
            #[allow(unused_assignments)]
            let mut utc_offset: Option<(char, i32, i32)> = None;
            // Try HH:MM:SS
            if let Some(caps) = MATCH_HMS.captures(ts) {
                hour = caps[1].parse().ok()?;
                minute = caps[2].parse().ok()?;
                second = caps[3].parse().ok()?;
                if let Some(frac) = caps.get(4) {
                    millisecond = format!("0.{}", frac.as_str()).parse::<f64>().ok()? * 1000.0;
                }

                // Validate basic vs extended
                let dash_count = ts.matches(':').count();
                if dash_count > 0 && dash_count != 2 && dash_count != 3 {
                    return None;
                }

                // Extract offset info (group 5=sign, 6=hours, 7=minutes)
                let sign = caps.get(5).map_or(' ', |m| m.as_str().chars().next().unwrap());
                let off_h: i32 = caps.get(6).map_or(0, |m| m.as_str().parse().unwrap_or(0));
                let off_m: i32 = caps.get(7).map_or(0, |m| m.as_str().parse().unwrap_or(0));
                utc_offset = Some((sign, off_h, off_m));
            } else if let Some(caps) = MATCH_HM.captures(ts) {
                hour = caps[1].parse().ok()?;
                minute = caps[2].parse().ok()?;
                if let Some(frac) = caps.get(3) {
                    second = (format!("0.{}", frac.as_str()).parse::<f64>().ok()? * 60.0) as i32;
                    millisecond =
                        (format!("0.{}", frac.as_str()).parse::<f64>().ok()? * 60.0 * 1000.0)
                            % 1000.0;
                }

                let dash_count = ts.matches(':').count();
                if dash_count > 2 {
                    return None;
                }

                // Extract offset info (group 4=sign, 5=hours, 6=minutes)
                let sign = caps.get(4).map_or(' ', |m| m.as_str().chars().next().unwrap());
                let off_h: i32 = caps.get(5).map_or(0, |m| m.as_str().parse().unwrap_or(0));
                let off_m: i32 = caps.get(6).map_or(0, |m| m.as_str().parse().unwrap_or(0));
                utc_offset = Some((sign, off_h, off_m));
            } else if let Some(caps) = MATCH_H.captures(ts) {
                hour = caps[1].parse().ok()?;
                if let Some(frac) = caps.get(2) {
                    let frac_val: f64 = format!("0.{}", frac.as_str()).parse().ok()?;
                    minute = (frac_val * 60.0) as i32;
                    let remaining_minutes = frac_val * 60.0 - minute as f64;
                    second = (remaining_minutes * 60.0) as i32;
                    millisecond = (remaining_minutes * 60.0 - second as f64) * 1000.0;
                }

                // Extract offset info (group 3=sign, 4=hours, 5=minutes)
                let sign = caps.get(3).map_or(' ', |m| m.as_str().chars().next().unwrap());
                let off_h: i32 = caps.get(4).map_or(0, |m| m.as_str().parse().unwrap_or(0));
                let off_m: i32 = caps.get(5).map_or(0, |m| m.as_str().parse().unwrap_or(0));
                utc_offset = Some((sign, off_h, off_m));
            } else {
                return None;
            }

            // Validate time ranges (on raw parsed values, before UTC offset — matching JS)
            if minute >= 60
                || second >= 61
                || hour > 24
                || (hour == 24 && (minute > 0 || second > 0 || millisecond > 0.0))
            {
                return None;
            }

            // Now apply UTC offset (after validation, matching JS order)
            if let Some((sign, off_h, off_m)) = utc_offset {
                match sign {
                    '+' => { hour -= off_h; minute -= off_m; }
                    '-' => { hour += off_h; minute += off_m; }
                    _ => {} // 'Z' or no offset = UTC
                }
            }
        }

        // Handle leap second (second == 60)
        let is_leap_second = second == 60;
        if is_leap_second {
            second -= 1;
        }

        // Normalize
        while minute >= 60 {
            minute -= 60;
            hour += 1;
        }
        while hour >= 24 {
            hour -= 24;
            day += 1;
        }
        let mut tmp = if in_leap_year && month == 2 {
            DAYS_IN_LEAP_FEBRUARY
        } else {
            DAYS_IN_MONTH[(month - 1) as usize]
        };
        while day > tmp {
            day -= tmp;
            month += 1;
            if month > 12 {
                month -= 12;
                year += 1;
            }
            tmp = if in_leap_year && month == 2 {
                DAYS_IN_LEAP_FEBRUARY
            } else {
                DAYS_IN_MONTH[(month - 1) as usize]
            };
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
            tmp = if in_leap_year && month == 2 {
                DAYS_IN_LEAP_FEBRUARY
            } else {
                DAYS_IN_MONTH[(month - 1) as usize]
            };
            day += tmp;
        }

        let (dn, sod) = compute_julian_date_components(
            year, month, day, hour, minute, second, millisecond,
        );
        let mut result = Self::new(dn as f64, sod, TimeStandard::UTC);

        if is_leap_second {
            result = Self::add_seconds_new(&result, 1.0);
        }

        Some(result)
    }

    // ── to_gregorian_date ────────────────────────────────────────────────

    /// Converts this `JulianDate` to a [`GregorianDate`].
    pub fn to_gregorian_date(&self) -> GregorianDate {
        let mut scratch = JulianDate {
            day_number: 0,
            seconds_of_day: 0.0,
        };
        let mut is_leap_second = false;

        let this_utc = match convert_tai_to_utc(self, &mut scratch) {
            Some(()) => &scratch,
            None => {
                // During a leap second
                let tmp = JulianDate::add_seconds_new(self, -1.0);
                let _ = convert_tai_to_utc(&tmp, &mut scratch);
                is_leap_second = true;
                &scratch
            }
        };

        let mut julian_day_number = this_utc.day_number;
        let seconds_of_day = this_utc.seconds_of_day;

        if seconds_of_day >= 43200.0 {
            julian_day_number += 1;
        }

        // Algorithm from page 604 of the Explanatory Supplement to the
        // Astronomical Almanac (Seidelmann 1992).
        let mut l = (julian_day_number + 68569) as i64;
        let n = (4 * l) / 146097;
        l -= (146097 * n + 3) / 4;
        let i = (4000 * (l + 1)) / 1461001;
        l = l - (1461 * i) / 4 + 31;
        let j = (80 * l) / 2447;
        let day = (l - (2447 * j) / 80) as i32;
        l = j / 11;
        let month = (j + 2 - 12 * l) as i32;
        let year = (100 * (n - 49) + i + l) as i32;

        let mut hour = (seconds_of_day / SECONDS_PER_HOUR) as i32;
        let mut remaining = seconds_of_day - hour as f64 * SECONDS_PER_HOUR;
        let minute = (remaining / SECONDS_PER_MINUTE) as i32;
        remaining -= minute as f64 * SECONDS_PER_MINUTE;
        let mut second = remaining as i32;
        let millisecond =
            (remaining - second as f64) / SECONDS_PER_MILLISECOND;

        // JulianDates are noon-based
        hour += 12;
        if hour > 23 {
            hour -= 24;
        }

        if is_leap_second {
            second += 1;
        }

        GregorianDate::new(
            year, month, day, hour, minute, second, millisecond, is_leap_second,
        )
    }

    // ── to_iso8601 ───────────────────────────────────────────────────────

    /// Formats this `JulianDate` as an ISO 8601 string.
    pub fn to_iso8601(&self, precision: Option<usize>) -> String {
        let gd = self.to_gregorian_date();
        let mut year = gd.year;
        let mut month = gd.month;
        let mut day = gd.day;
        let mut hour = gd.hour;
        let minute = gd.minute;
        let second = gd.second;
        let millisecond = gd.millisecond;

        // Special case: 10000-01-01T00:00:00 == 9999-12-31T24:00:00
        if year == 10000 && month == 1 && day == 1 && hour == 0 && minute == 0 && second == 0 && millisecond == 0.0 {
            year = 9999;
            month = 12;
            day = 31;
            hour = 24;
        }

        let date_part = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            year, month, day, hour, minute, second
        );

        match precision {
            Some(0) => format!("{}Z", date_part),
            Some(prec) => {
                // Convert milliseconds to fractional seconds: ms / 1000
                let frac = millisecond / 1000.0;
                let frac_str = format!("{:.*}", prec, frac);
                // Remove leading "0." to get the digit string
                let digits = frac_str.trim_start_matches("0.");
                let digits = &digits[..prec.min(digits.len())];
                // Pad with trailing zeros if needed
                let padded = format!("{:<0width$}", digits, width = prec);
                format!("{}.{}Z", date_part, padded)
            }
            None => {
                if millisecond == 0.0 {
                    format!("{}Z", date_part)
                } else {
                    // Convert milliseconds to fractional seconds
                    let frac = millisecond / 1000.0;
                    if frac < 1e-6 {
                        let frac_str = format!("{:.20}", frac);
                        let digits = frac_str.trim_start_matches("0.").trim_end_matches('0');
                        format!("{}.{}Z", date_part, digits)
                    } else {
                        let frac_str = format!("{}", frac);
                        let digits = frac_str.trim_start_matches("0.");
                        format!("{}.{}Z", date_part, digits)
                    }
                }
            }
        }
    }

    // ── arithmetic ───────────────────────────────────────────────────────

    fn add_seconds_mut(julian_date: &mut JulianDate, seconds: f64) {
        set_components(
            julian_date.day_number,
            julian_date.seconds_of_day + seconds,
            julian_date,
        );
    }

    /// Returns a new `JulianDate` with `seconds` added.
    pub fn add_seconds(julian_date: &JulianDate, seconds: f64) -> JulianDate {
        Self::add_seconds_new(julian_date, seconds)
    }

    /// Returns a new `JulianDate` with `seconds` added.
    pub fn add_seconds_new(julian_date: &JulianDate, seconds: f64) -> JulianDate {
        let mut result = JulianDate {
            day_number: 0,
            seconds_of_day: 0.0,
        };
        set_components(
            julian_date.day_number,
            julian_date.seconds_of_day + seconds,
            &mut result,
        );
        result
    }

    /// Returns a new `JulianDate` with `minutes` added.
    pub fn add_minutes(julian_date: &JulianDate, minutes: f64) -> JulianDate {
        let mut result = JulianDate {
            day_number: 0,
            seconds_of_day: 0.0,
        };
        set_components(
            julian_date.day_number,
            julian_date.seconds_of_day + minutes * SECONDS_PER_MINUTE,
            &mut result,
        );
        result
    }

    /// Returns a new `JulianDate` with `hours` added.
    pub fn add_hours(julian_date: &JulianDate, hours: f64) -> JulianDate {
        let mut result = JulianDate {
            day_number: 0,
            seconds_of_day: 0.0,
        };
        set_components(
            julian_date.day_number,
            julian_date.seconds_of_day + hours * SECONDS_PER_HOUR,
            &mut result,
        );
        result
    }

    /// Returns a new `JulianDate` with `days` added.
    pub fn add_days(julian_date: &JulianDate, days: f64) -> JulianDate {
        let mut result = JulianDate {
            day_number: 0,
            seconds_of_day: 0.0,
        };
        set_components(
            julian_date.day_number + days as i32,
            julian_date.seconds_of_day,
            &mut result,
        );
        result
    }

    // ── comparison ───────────────────────────────────────────────────────

    /// Compares two instances. Returns negative if `left < right`, positive if
    /// `left > right`, or zero if equal.
    pub fn compare(left: &JulianDate, right: &JulianDate) -> i32 {
        let day_diff = left.day_number - right.day_number;
        if day_diff != 0 {
            return day_diff;
        }
        let sod_diff = left.seconds_of_day - right.seconds_of_day;
        if sod_diff < 0.0 {
            -1
        } else if sod_diff > 0.0 {
            1
        } else {
            0
        }
    }

    /// Returns `true` if the two dates are exactly equal.
    pub fn equals(left: &JulianDate, right: &JulianDate) -> bool {
        left.day_number == right.day_number && left.seconds_of_day == right.seconds_of_day
    }

    /// Returns `true` if the two dates are within `epsilon` seconds of each other.
    pub fn equals_epsilon(left: &JulianDate, right: &JulianDate, epsilon: f64) -> bool {
        Self::seconds_difference(left, right).abs() <= epsilon
    }

    /// Returns `true` if `left` is earlier than `right`.
    pub fn less_than(left: &JulianDate, right: &JulianDate) -> bool {
        Self::compare(left, right) < 0
    }

    /// Returns `true` if `left` is earlier than or equal to `right`.
    pub fn less_than_or_equals(left: &JulianDate, right: &JulianDate) -> bool {
        Self::compare(left, right) <= 0
    }

    /// Returns `true` if `left` is later than `right`.
    pub fn greater_than(left: &JulianDate, right: &JulianDate) -> bool {
        Self::compare(left, right) > 0
    }

    /// Returns `true` if `left` is later than or equal to `right`.
    pub fn greater_than_or_equals(left: &JulianDate, right: &JulianDate) -> bool {
        Self::compare(left, right) >= 0
    }

    // ── differences ──────────────────────────────────────────────────────

    /// Computes the difference in seconds (`left - right`).
    pub fn seconds_difference(left: &JulianDate, right: &JulianDate) -> f64 {
        let day_diff =
            (left.day_number - right.day_number) as f64 * SECONDS_PER_DAY;
        day_diff + (left.seconds_of_day - right.seconds_of_day)
    }

    /// Computes the difference in days (`left - right`).
    pub fn days_difference(left: &JulianDate, right: &JulianDate) -> f64 {
        let day_diff = (left.day_number - right.day_number) as f64;
        let second_diff =
            (left.seconds_of_day - right.seconds_of_day) / SECONDS_PER_DAY;
        day_diff + second_diff
    }

    /// Computes the total number of whole and fractional days.
    pub fn total_days(julian_date: &JulianDate) -> f64 {
        julian_date.day_number as f64
            + julian_date.seconds_of_day / SECONDS_PER_DAY
    }

    // ── TAI/UTC ──────────────────────────────────────────────────────────

    /// Computes the number of seconds TAI is ahead of UTC at the given date.
    pub fn compute_tai_minus_utc(julian_date: &JulianDate) -> f64 {
        let leap_seconds = leap_seconds_table();
        let mut index = binary_search(leap_seconds, julian_date, |ls, jd| {
            compare_leap_second_dates(ls, jd)
        });
        if index < 0 {
            index = !index;
            index -= 1;
            if index < 0 {
                index = 0;
            }
        }
        leap_seconds[index as usize].offset
    }

    // ── clone ────────────────────────────────────────────────────────────

    /// Duplicates this instance.
    pub fn clone_instance(&self) -> JulianDate {
        JulianDate {
            day_number: self.day_number,
            seconds_of_day: self.seconds_of_day,
        }
    }
}

impl PartialEq for JulianDate {
    fn eq(&self, other: &Self) -> bool {
        Self::equals(self, other)
    }
}

impl std::fmt::Display for JulianDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_iso8601(None))
    }
}
