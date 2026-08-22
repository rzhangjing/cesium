//! Ported from `packages/engine/Source/Core/SteppedSpline.js`.

use crate::spline::{clamp_time, find_time_interval, wrap_time, SplinePoint};

/// A spline that is composed of piecewise constants representing a step function.
pub struct SteppedSpline {
    times: Vec<f64>,
    points: Vec<SplinePoint>,
    last_time_index: usize,
}

impl SteppedSpline {
    /// Creates a new SteppedSpline.
    pub fn new(times: Vec<f64>, points: Vec<SplinePoint>) -> Self {
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
    pub fn points(&self) -> &[SplinePoint] {
        &self.points
    }

    /// Finds the time interval containing the given time.
    pub fn find_time_interval(&self, time: f64) -> Option<usize> {
        find_time_interval(&self.times, time, Some(self.last_time_index))
    }

    /// Wraps the given time to the period covered by the spline.
    pub fn wrap_time(&self, time: f64) -> f64 {
        wrap_time(&self.times, time)
    }

    /// Clamps the given time to the period covered by the spline.
    pub fn clamp_time(&self, time: f64) -> f64 {
        clamp_time(&self.times, time)
    }

    /// Evaluates the curve at a given time (returns the step value).
    pub fn evaluate(&mut self, time: f64) -> Option<SplinePoint> {
        let i = self.find_time_interval(time)?;
        self.last_time_index = i;
        Some(self.points[i].clone_point())
    }
}
