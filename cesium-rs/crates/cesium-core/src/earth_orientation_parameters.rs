//! Ported from `packages/engine/Source/Core/EarthOrientationParameters.js`.
//!
//! Specifies Earth polar motion coordinates and the difference between UT1 and UTC.

use crate::binary_search::binary_search;
use crate::earth_orientation_parameters_sample::EarthOrientationParametersSample;
use crate::julian_date::JulianDate;
use crate::time_constants::MODIFIED_JULIAN_DATE_DIFFERENCE;
use crate::time_standard::TimeStandard;

/// EOP data structure as loaded from JSON.
#[derive(Debug, Clone)]
pub struct EopData {
    pub column_names: Vec<String>,
    pub samples: Vec<f64>,
}

/// Earth Orientation Parameters for ICRF → ITRF transformation.
pub struct EarthOrientationParameters {
    dates: Vec<JulianDate>,
    samples: Vec<f64>,
    date_column: usize,
    x_pole_wander_column: usize,
    y_pole_wander_column: usize,
    ut1_minus_utc_column: usize,
    x_celestial_pole_offset_column: usize,
    y_celestial_pole_offset_column: usize,
    tai_minus_utc_column: usize,
    column_count: usize,
    last_index: Option<usize>,
    add_new_leap_seconds: bool,
}

impl EarthOrientationParameters {
    /// Creates a new `EarthOrientationParameters` from EOP data.
    pub fn new(data: Option<EopData>, add_new_leap_seconds: Option<bool>) -> Self {
        let add_new = add_new_leap_seconds.unwrap_or(true);

        let mut eop = Self {
            dates: Vec::new(),
            samples: Vec::new(),
            date_column: 0,
            x_pole_wander_column: 0,
            y_pole_wander_column: 0,
            ut1_minus_utc_column: 0,
            x_celestial_pole_offset_column: 0,
            y_celestial_pole_offset_column: 0,
            tai_minus_utc_column: 0,
            column_count: 0,
            last_index: None,
            add_new_leap_seconds: add_new,
        };

        if let Some(eop_data) = data {
            eop.on_data_ready(eop_data);
        } else {
            // Use all zeros
            let default_data = EopData {
                column_names: vec![
                    "dateIso8601".into(),
                    "modifiedJulianDateUtc".into(),
                    "xPoleWanderRadians".into(),
                    "yPoleWanderRadians".into(),
                    "ut1MinusUtcSeconds".into(),
                    "lengthOfDayCorrectionSeconds".into(),
                    "xCelestialPoleOffsetRadians".into(),
                    "yCelestialPoleOffsetRadians".into(),
                    "taiMinusUtcSeconds".into(),
                ],
                samples: Vec::new(),
            };
            eop.on_data_ready(default_data);
        }

        eop
    }

    /// Computes the EOP for a given date by interpolation.
    pub fn compute(
        &self,
        date: &JulianDate,
        result: &mut EarthOrientationParametersSample,
    ) {
        if self.samples.is_empty() {
            result.x_pole_wander = 0.0;
            result.y_pole_wander = 0.0;
            result.x_pole_offset = 0.0;
            result.y_pole_offset = 0.0;
            result.ut1_minus_utc = 0.0;
            return;
        }

        let dates = &self.dates;
        let last_index = self.last_index;

        let (before, after);

        if let Some(li) = last_index {
            if li < dates.len() {
                let is_after_previous = JulianDate::less_than_or_equals(&dates[li], date);
                let is_after_last_sample = li + 1 >= dates.len();
                let is_before_next = is_after_last_sample
                    || JulianDate::greater_than_or_equals(&dates[li + 1], date);

                if is_after_previous && is_before_next {
                    before = li;
                    after = before + 1;
                    self.interpolate(dates, date, before, after, result);
                    return;
                }
            }
        }

        let index = binary_search(dates, date, |a, b| JulianDate::compare(a, b) as f64);
        if index >= 0 {
            let mut idx = index as usize;
            if idx + 1 < dates.len() && JulianDate::equals(&dates[idx + 1], date) {
                idx += 1;
            }
            before = idx;
            after = idx;
        } else {
            after = (!index) as usize;
            before = if after > 0 { after - 1 } else { 0 };
        }

        self.interpolate(dates, date, before, after, result);
    }

    fn interpolate(
        &self,
        dates: &[JulianDate],
        date: &JulianDate,
        before: usize,
        after: usize,
        result: &mut EarthOrientationParametersSample,
    ) {
        let cc = self.column_count;

        if after > dates.len() - 1 {
            result.x_pole_wander = 0.0;
            result.y_pole_wander = 0.0;
            result.x_pole_offset = 0.0;
            result.y_pole_offset = 0.0;
            result.ut1_minus_utc = 0.0;
            return;
        }

        if JulianDate::equals(&dates[before], &dates[after])
            || JulianDate::equals(date, &dates[before])
        {
            self.fill_from_index(before, cc, result);
            return;
        }
        if JulianDate::equals(date, &dates[after]) {
            self.fill_from_index(after, cc, result);
            return;
        }

        let factor = JulianDate::seconds_difference(date, &dates[before])
            / JulianDate::seconds_difference(&dates[after], &dates[before]);

        let start_before = before * cc;
        let start_after = after * cc;

        let mut before_ut1 = self.samples[start_before + self.ut1_minus_utc_column];
        let mut after_ut1 = self.samples[start_after + self.ut1_minus_utc_column];

        let offset_diff = after_ut1 - before_ut1;
        if offset_diff > 0.5 || offset_diff < -0.5 {
            let before_tai = self.samples[start_before + self.tai_minus_utc_column];
            let after_tai = self.samples[start_after + self.tai_minus_utc_column];
            if before_tai != after_tai {
                if JulianDate::equals(&dates[after], date) {
                    before_ut1 = after_ut1;
                } else {
                    after_ut1 -= after_tai - before_tai;
                }
            }
        }

        result.x_pole_wander = linear_interp(
            factor,
            self.samples[start_before + self.x_pole_wander_column],
            self.samples[start_after + self.x_pole_wander_column],
        );
        result.y_pole_wander = linear_interp(
            factor,
            self.samples[start_before + self.y_pole_wander_column],
            self.samples[start_after + self.y_pole_wander_column],
        );
        result.x_pole_offset = linear_interp(
            factor,
            self.samples[start_before + self.x_celestial_pole_offset_column],
            self.samples[start_after + self.x_celestial_pole_offset_column],
        );
        result.y_pole_offset = linear_interp(
            factor,
            self.samples[start_before + self.y_celestial_pole_offset_column],
            self.samples[start_after + self.y_celestial_pole_offset_column],
        );
        result.ut1_minus_utc = linear_interp(factor, before_ut1, after_ut1);
    }

    fn fill_from_index(
        &self,
        index: usize,
        _column_count: usize,
        result: &mut EarthOrientationParametersSample,
    ) {
        let start = index * self.column_count;
        result.x_pole_wander = self.samples[start + self.x_pole_wander_column];
        result.y_pole_wander = self.samples[start + self.y_pole_wander_column];
        result.x_pole_offset = self.samples[start + self.x_celestial_pole_offset_column];
        result.y_pole_offset = self.samples[start + self.y_celestial_pole_offset_column];
        result.ut1_minus_utc = self.samples[start + self.ut1_minus_utc_column];
    }

    fn on_data_ready(&mut self, eop_data: EopData) {
        let find_col = |name: &str| -> usize {
            eop_data
                .column_names
                .iter()
                .position(|n| n == name)
                .unwrap_or(usize::MAX)
        };

        self.date_column = find_col("modifiedJulianDateUtc");
        self.x_pole_wander_column = find_col("xPoleWanderRadians");
        self.y_pole_wander_column = find_col("yPoleWanderRadians");
        self.ut1_minus_utc_column = find_col("ut1MinusUtcSeconds");
        self.x_celestial_pole_offset_column = find_col("xCelestialPoleOffsetRadians");
        self.y_celestial_pole_offset_column = find_col("yCelestialPoleOffsetRadians");
        self.tai_minus_utc_column = find_col("taiMinusUtcSeconds");
        self.column_count = eop_data.column_names.len();

        self.samples = eop_data.samples;
        self.dates.clear();
        self.last_index = None;

        let mut last_tai_minus_utc: Option<f64> = None;

        let cc = self.column_count;
        let date_col = self.date_column;
        let tai_col = self.tai_minus_utc_column;

        let mut i = 0;
        while i < self.samples.len() {
            let mjd = self.samples[i + date_col];
            let tai_minus_utc = self.samples[i + tai_col];
            let day = mjd + MODIFIED_JULIAN_DATE_DIFFERENCE;
            let date = JulianDate::new(day, tai_minus_utc, TimeStandard::TAI);
            self.dates.push(date);

            if self.add_new_leap_seconds {
                if let Some(last_tai) = last_tai_minus_utc {
                    if tai_minus_utc != last_tai {
                        // Leap second boundary – handled by the JulianDate table
                    }
                }
                last_tai_minus_utc = Some(tai_minus_utc);
            }

            i += cc;
        }
    }
}

fn linear_interp(dx: f64, y1: f64, y2: f64) -> f64 {
    y1 + dx * (y2 - y1)
}
