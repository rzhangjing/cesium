//! Ported from `packages/engine/Source/Core/Iau2006XysData.js`.
//!
//! A set of IAU 2006 XYS data used to evaluate the transformation between
//! the International Celestial Reference Frame (ICRF) and the International
//! Terrestrial Reference Frame (ITRF).

use crate::iau2006_xys_sample::Iau2006XysSample;
use crate::julian_date::JulianDate;
use crate::time_standard::TimeStandard;

/// IAU 2006 XYS data for ICRF ↔ ITRF transformation.
pub struct Iau2006XysData {
    _xys_file_url_template: Option<String>,
    interpolation_order: usize,
    _sample_zero_julian_ephemeris_date: f64,
    sample_zero_date_tt: JulianDate,
    step_size_days: f64,
    _samples_per_xys_file: usize,
    total_samples: usize,
    samples: Vec<Option<f64>>,
    _chunk_downloads_in_progress: Vec<Option<std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>>>,
    denominators: Vec<f64>,
    x_table: Vec<f64>,
    work: Vec<f64>,
    coef: Vec<f64>,
}

/// Options for constructing [`Iau2006XysData`].
pub struct Iau2006XysDataOptions {
    pub xys_file_url_template: Option<String>,
    pub interpolation_order: Option<usize>,
    pub sample_zero_julian_ephemeris_date: Option<f64>,
    pub step_size_days: Option<f64>,
    pub samples_per_xys_file: Option<usize>,
    pub total_samples: Option<usize>,
}

impl Default for Iau2006XysDataOptions {
    fn default() -> Self {
        Self {
            xys_file_url_template: None,
            interpolation_order: None,
            sample_zero_julian_ephemeris_date: None,
            step_size_days: None,
            samples_per_xys_file: None,
            total_samples: None,
        }
    }
}

impl Iau2006XysData {
    /// Creates a new `Iau2006XysData` with the given options.
    pub fn new(options: Option<Iau2006XysDataOptions>) -> Self {
        let opts = options.unwrap_or_default();

        let interpolation_order = opts.interpolation_order.unwrap_or(9);
        let sample_zero_jed = opts.sample_zero_julian_ephemeris_date.unwrap_or(2442396.5);
        let sample_zero_date_tt =
            JulianDate::new(sample_zero_jed, 0.0, TimeStandard::TAI);
        let step_size_days = opts.step_size_days.unwrap_or(1.0);
        let samples_per_xys_file = opts.samples_per_xys_file.unwrap_or(1000);
        let total_samples = opts.total_samples.unwrap_or(27426);

        let samples = vec![None; total_samples * 3];

        let order = interpolation_order;
        let mut denominators = vec![0.0; order + 1];
        let mut x_table = vec![0.0; order + 1];

        let step_n = step_size_days.powi(order as i32);

        for i in 0..=order {
            let mut denom = step_n;
            x_table[i] = i as f64 * step_size_days;

            for j in 0..=order {
                if j != i {
                    denom *= (i as f64) - (j as f64);
                }
            }
            denominators[i] = 1.0 / denom;
        }

        let work = vec![0.0; order + 1];
        let coef = vec![0.0; order + 1];

        Self {
            _xys_file_url_template: opts.xys_file_url_template,
            interpolation_order: order,
            _sample_zero_julian_ephemeris_date: sample_zero_jed,
            sample_zero_date_tt,
            step_size_days,
            _samples_per_xys_file: samples_per_xys_file,
            total_samples,
            samples,
            _chunk_downloads_in_progress: Vec::new(),
            denominators,
            x_table,
            work,
            coef,
        }
    }

    /// Computes the XYS values for a given date by interpolation.
    pub fn compute_xys_radians(
        &mut self,
        day_tt: i64,
        second_tt: f64,
        result: &mut Option<Iau2006XysSample>,
    ) -> Option<Iau2006XysSample> {
        let _date_tt = JulianDate::new(0.0, 0.0, TimeStandard::TAI);
        let days_since_epoch = {
            let d = JulianDate::new(day_tt as f64, second_tt, TimeStandard::TAI);
            JulianDate::days_difference(&d, &self.sample_zero_date_tt)
        };

        if days_since_epoch < 0.0 {
            return None;
        }

        let center_index = (days_since_epoch / self.step_size_days) as usize;
        if center_index >= self.total_samples {
            return None;
        }

        let degree = self.interpolation_order;

        let mut first_index = center_index as isize - (degree / 2) as isize;
        if first_index < 0 {
            first_index = 0;
        }
        let mut first_index = first_index as usize;
        let mut last_index = first_index + degree;
        if last_index >= self.total_samples {
            last_index = self.total_samples - 1;
            first_index = last_index - degree;
        }

        // Check if data is available
        if self.samples[first_index * 3].is_none()
            || self.samples[last_index * 3].is_none()
        {
            return None;
        }

        let mut sample = result.take().unwrap_or(Iau2006XysSample::new(0.0, 0.0, 0.0));
        sample.x = 0.0;
        sample.y = 0.0;
        sample.s = 0.0;

        let x = days_since_epoch - first_index as f64 * self.step_size_days;

        for i in 0..=degree {
            self.work[i] = x - self.x_table[i];
        }

        for i in 0..=degree {
            self.coef[i] = 1.0;
            for j in 0..=degree {
                if j != i {
                    self.coef[i] *= self.work[j];
                }
            }
            self.coef[i] *= self.denominators[i];

            let sample_index = (first_index + i) * 3;
            sample.x += self.coef[i] * self.samples[sample_index].unwrap_or(0.0);
            sample.y += self.coef[i] * self.samples[sample_index + 1].unwrap_or(0.0);
            sample.s += self.coef[i] * self.samples[sample_index + 2].unwrap_or(0.0);
        }

        Some(sample)
    }
}
