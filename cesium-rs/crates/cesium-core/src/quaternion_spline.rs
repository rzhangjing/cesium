//! Ported from `packages/engine/Source/Core/QuaternionSpline.js`.

use crate::quaternion::Quaternion;
use crate::spline::{clamp_time, find_time_interval, wrap_time};

/// A spline that interpolates between Quaternion control points using SLERP.
pub struct QuaternionSpline {
    times: Vec<f64>,
    points: Vec<Quaternion>,
    last_time_index: usize,
}

impl QuaternionSpline {
    /// Creates a new QuaternionSpline.
    pub fn new(times: Vec<f64>, points: Vec<Quaternion>) -> Self {
        Self {
            times,
            points,
            last_time_index: 0,
        }
    }

    /// Returns the times array.
    pub fn times(&self) -> &[f64] {
        &self.times
    }

    /// Returns the points array.
    pub fn points(&self) -> &[Quaternion] {
        &self.points
    }

    /// Wraps time.
    pub fn wrap_time(&self, time: f64) -> f64 {
        wrap_time(&self.times, time)
    }

    /// Clamps time.
    pub fn clamp_time(&self, time: f64) -> f64 {
        clamp_time(&self.times, time)
    }

    /// Evaluates the curve at a given time using SLERP.
    pub fn evaluate(&mut self, time: f64, result: &mut Quaternion) -> Option<()> {
        let i = find_time_interval(&self.times, time, Some(self.last_time_index))?;
        self.last_time_index = i;

        let u = (time - self.times[i]) / (self.times[i + 1] - self.times[i]);
        Quaternion::slerp(&self.points[i], &self.points[i + 1], u, result);
        Some(())
    }
}
