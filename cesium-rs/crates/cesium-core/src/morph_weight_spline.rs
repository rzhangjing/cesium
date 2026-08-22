//! Ported from `packages/engine/Source/Core/MorphWeightSpline.js`.

use crate::spline::{clamp_time, find_time_interval, wrap_time};

/// A spline that interpolates between an array of morph target weights.
pub struct MorphWeightSpline {
    times: Vec<f64>,
    weights: Vec<Vec<f64>>,
    last_time_index: usize,
}

impl MorphWeightSpline {
    /// Creates a new MorphWeightSpline.
    pub fn new(times: Vec<f64>, weights: Vec<Vec<f64>>) -> Self {
        Self {
            times,
            weights,
            last_time_index: 0,
        }
    }

    /// Returns the times array.
    pub fn times(&self) -> &[f64] {
        &self.times
    }

    /// Returns the weights array.
    pub fn weights(&self) -> &[Vec<f64>] {
        &self.weights
    }

    /// Wraps time.
    pub fn wrap_time(&self, time: f64) -> f64 {
        wrap_time(&self.times, time)
    }

    /// Clamps time.
    pub fn clamp_time(&self, time: f64) -> f64 {
        clamp_time(&self.times, time)
    }

    /// Evaluates the curve at a given time using Hermite interpolation.
    pub fn evaluate(&mut self, time: f64, result: Option<&mut Vec<f64>>) -> Option<Vec<f64>> {
        let i = find_time_interval(&self.times, time, Some(self.last_time_index))?;
        self.last_time_index = i;

        let t = (time - self.times[i]) / (self.times[i + 1] - self.times[i]);
        let t2 = t * t;
        let t3 = t2 * t;

        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let _h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let _h11 = t3 - t2;

        let length = self.weights[i].len();
        let mut r = result.cloned().unwrap_or_else(|| vec![0.0; length]);

        for j in 0..length {
            r[j] = h00 * self.weights[i][j]
                + h01 * self.weights[i + 1][j];
            // Tangents are zero for morph weights (natural cubic)
        }

        Some(r)
    }
}
