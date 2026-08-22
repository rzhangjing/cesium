//! Ported from `packages/engine/Source/Core/ConstantSpline.js`.

use crate::spline::SplinePoint;

/// A spline that evaluates to a constant value.
pub struct ConstantSpline {
    value: SplinePoint,
}

impl ConstantSpline {
    /// Creates a new ConstantSpline.
    pub fn new(value: SplinePoint) -> Self {
        Self { value }
    }

    /// Returns the constant value.
    pub fn value(&self) -> &SplinePoint {
        &self.value
    }

    /// Wraps time (always returns 0.0 for a constant spline).
    pub fn wrap_time(&self, _time: f64) -> f64 {
        0.0
    }

    /// Clamps time (always returns 0.0 for a constant spline).
    pub fn clamp_time(&self, _time: f64) -> f64 {
        0.0
    }

    /// Evaluates the curve (returns the constant value).
    pub fn evaluate(&self, _time: f64) -> SplinePoint {
        self.value.clone_point()
    }
}
