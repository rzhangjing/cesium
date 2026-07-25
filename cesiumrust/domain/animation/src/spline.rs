//! Spline interpolation system.
//!
//! Maps to CesiumJS:
//! - `Core/Spline.js` (base)
//! - `Core/LinearSpline.js`
//! - `Core/CatmullRomSpline.js`
//! - `Core/HermiteSpline.js`
//! - `Core/QuaternionSpline.js`
//! - `Core/SteppedSpline.js` (via SteppedSpline in CesiumJS)
//! - `Core/ConstantSpline.js` (via MorphWeightSpline)

use glam::{DQuat, DVec3};

// ============================================================================
// Spline trait
// ============================================================================

/// Common spline operations.
pub trait Spline {
    /// Get the time values.
    fn times(&self) -> &[f64];

    /// Find the time interval index for a given time.
    /// Returns index i such that times[i] <= time <= times[i+1].
    fn find_time_interval(&self, time: f64) -> usize {
        let times = self.times();
        if times.is_empty() {
            return 0;
        }
        if time <= times[0] {
            return 0;
        }
        let last = times.len() - 1;
        if time >= times[last] {
            return last.saturating_sub(1);
        }
        // Binary search
        let mut lo = 0;
        let mut hi = last;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if times[mid] <= time && time < times[mid + 1] {
                return mid;
            } else if times[mid] > time {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        lo.min(last.saturating_sub(1))
    }

    /// Wrap time to the spline's period.
    fn wrap_time(&self, time: f64) -> f64 {
        let times = self.times();
        if times.len() < 2 {
            return time;
        }
        let start = times[0];
        let end = times[times.len() - 1];
        let duration = end - start;
        if duration <= 0.0 {
            return start;
        }
        let mut t = (time - start) % duration;
        if t < 0.0 {
            t += duration;
        }
        start + t
    }

    /// Clamp time to the spline's range.
    fn clamp_time(&self, time: f64) -> f64 {
        let times = self.times();
        if times.is_empty() {
            return time;
        }
        time.clamp(times[0], times[times.len() - 1])
    }
}

// ============================================================================
// LinearSpline
// ============================================================================

/// Piecewise linear interpolation spline.
///
/// Maps to CesiumJS `Core/LinearSpline.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearSpline {
    /// Time values (strictly increasing).
    pub times: Vec<f64>,
    /// Control points.
    pub points: Vec<DVec3>,
}

impl LinearSpline {
    /// Create a new linear spline.
    ///
    /// # Panics
    /// Panics if points.len() < 2 or times.len() != points.len().
    pub fn new(times: Vec<f64>, points: Vec<DVec3>) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");
        Self { times, points }
    }

    /// Evaluate the spline at a given time.
    pub fn evaluate(&self, time: f64) -> DVec3 {
        let i = self.find_time_interval(time);
        let t0 = self.times[i];
        let t1 = self.times[i + 1];
        let u = if (t1 - t0).abs() > 1e-15 {
            (time - t0) / (t1 - t0)
        } else {
            0.0
        };
        self.points[i].lerp(self.points[i + 1], u)
    }
}

impl Spline for LinearSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// CatmullRomSpline
// ============================================================================

/// Catmull-Rom spline for smooth C1-continuous curves.
///
/// Maps to CesiumJS `Core/CatmullRomSpline.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct CatmullRomSpline {
    /// Time values.
    pub times: Vec<f64>,
    /// Control points.
    pub points: Vec<DVec3>,
    /// Tangent at the first point.
    pub first_tangent: DVec3,
    /// Tangent at the last point.
    pub last_tangent: DVec3,
}

impl CatmullRomSpline {
    /// Create a new Catmull-Rom spline.
    ///
    /// # Panics
    /// Panics if points.len() < 2 or times.len() != points.len().
    pub fn new(times: Vec<f64>, points: Vec<DVec3>) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");

        // Compute default tangents
        let first_tangent = if points.len() >= 3 {
            (points[1] - points[0]) * 0.5
        } else {
            points[1] - points[0]
        };

        let n = points.len();
        let last_tangent = if n >= 3 {
            (points[n - 1] - points[n - 2]) * 0.5
        } else {
            points[n - 1] - points[n - 2]
        };

        Self {
            times,
            points,
            first_tangent,
            last_tangent,
        }
    }

    /// Evaluate the spline at a given time.
    pub fn evaluate(&self, time: f64) -> DVec3 {
        let n = self.points.len();
        if n == 2 {
            // Fall back to linear
            let i = self.find_time_interval(time);
            let t0 = self.times[i];
            let t1 = self.times[i + 1];
            let u = if (t1 - t0).abs() > 1e-15 {
                (time - t0) / (t1 - t0)
            } else {
                0.0
            };
            return self.points[0].lerp(self.points[1], u);
        }

        let i = self.find_time_interval(time);
        let t0 = self.times[i];
        let t1 = self.times[i + 1];
        let u = if (t1 - t0).abs() > 1e-15 {
            (time - t0) / (t1 - t0)
        } else {
            0.0
        };

        // Hermite basis functions
        let u2 = u * u;
        let u3 = u2 * u;
        let h00 = 2.0 * u3 - 3.0 * u2 + 1.0;
        let h10 = u3 - 2.0 * u2 + u;
        let h01 = -2.0 * u3 + 3.0 * u2;
        let h11 = u3 - u2;

        let (p0, p1, m0, m1) = if i == 0 {
            let p0 = self.points[0];
            let p1 = self.points[1];
            let m0 = self.first_tangent;
            let m1 = (self.points[2] - self.points[0]) * 0.5;
            (p0, p1, m0, m1)
        } else if i == n - 2 {
            let p0 = self.points[i];
            let p1 = self.points[i + 1];
            let m0 = (self.points[i + 1] - self.points[i - 1]) * 0.5;
            let m1 = self.last_tangent;
            (p0, p1, m0, m1)
        } else {
            let p0 = self.points[i];
            let p1 = self.points[i + 1];
            let m0 = (self.points[i + 1] - self.points[i - 1]) * 0.5;
            let m1 = (self.points[i + 2] - self.points[i]) * 0.5;
            (p0, p1, m0, m1)
        };

        p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11
    }
}

impl Spline for CatmullRomSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// HermiteSpline
// ============================================================================

/// Hermite spline with explicit tangents.
///
/// Maps to CesiumJS `Core/HermiteSpline.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct HermiteSpline {
    /// Time values.
    pub times: Vec<f64>,
    /// Control points.
    pub points: Vec<DVec3>,
    /// In-tangents at each point.
    pub in_tangents: Vec<DVec3>,
    /// Out-tangents at each point.
    pub out_tangents: Vec<DVec3>,
}

impl HermiteSpline {
    /// Create a new Hermite spline.
    pub fn new(
        times: Vec<f64>,
        points: Vec<DVec3>,
        in_tangents: Vec<DVec3>,
        out_tangents: Vec<DVec3>,
    ) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");
        assert_eq!(in_tangents.len(), points.len(), "in_tangents must match");
        assert_eq!(out_tangents.len(), points.len(), "out_tangents must match");
        Self {
            times,
            points,
            in_tangents,
            out_tangents,
        }
    }

    /// Evaluate the spline at a given time.
    pub fn evaluate(&self, time: f64) -> DVec3 {
        let i = self.find_time_interval(time);
        let t0 = self.times[i];
        let t1 = self.times[i + 1];
        let u = if (t1 - t0).abs() > 1e-15 {
            (time - t0) / (t1 - t0)
        } else {
            0.0
        };

        let u2 = u * u;
        let u3 = u2 * u;
        let h00 = 2.0 * u3 - 3.0 * u2 + 1.0;
        let h10 = u3 - 2.0 * u2 + u;
        let h01 = -2.0 * u3 + 3.0 * u2;
        let h11 = u3 - u2;

        let p0 = self.points[i];
        let p1 = self.points[i + 1];
        let m0 = self.out_tangents[i];
        let m1 = self.in_tangents[i + 1];

        p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11
    }
}

impl Spline for HermiteSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// QuaternionSpline
// ============================================================================

/// Quaternion spline using SLERP interpolation.
///
/// Maps to CesiumJS `Core/QuaternionSpline.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct QuaternionSpline {
    /// Time values.
    pub times: Vec<f64>,
    /// Quaternion control points.
    pub points: Vec<DQuat>,
}

impl QuaternionSpline {
    /// Create a new quaternion spline.
    ///
    /// # Panics
    /// Panics if points.len() < 2 or times.len() != points.len().
    pub fn new(times: Vec<f64>, points: Vec<DQuat>) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");
        Self { times, points }
    }

    /// Evaluate the spline at a given time using SLERP.
    pub fn evaluate(&self, time: f64) -> DQuat {
        let i = self.find_time_interval(time);
        let t0 = self.times[i];
        let t1 = self.times[i + 1];
        let u = if (t1 - t0).abs() > 1e-15 {
            (time - t0) / (t1 - t0)
        } else {
            0.0
        };
        self.points[i].slerp(self.points[i + 1], u)
    }
}

impl Spline for QuaternionSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// SteppedSpline
// ============================================================================

/// Stepped (piecewise constant) spline - holds value until next keyframe.
///
/// Maps to CesiumJS stepped interpolation behavior.
#[derive(Debug, Clone, PartialEq)]
pub struct SteppedSpline {
    /// Time values.
    pub times: Vec<f64>,
    /// Control points.
    pub points: Vec<DVec3>,
}

impl SteppedSpline {
    /// Create a new stepped spline.
    pub fn new(times: Vec<f64>, points: Vec<DVec3>) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");
        Self { times, points }
    }

    /// Evaluate the spline at a given time (returns previous keyframe value).
    pub fn evaluate(&self, time: f64) -> DVec3 {
        let i = self.find_time_interval(time);
        self.points[i]
    }
}

impl Spline for SteppedSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// ConstantSpline
// ============================================================================

/// Constant spline - always returns the same value.
///
/// Maps to CesiumJS `Core/ConstantSpline.js` / MorphWeightSpline.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstantSpline {
    /// The constant value.
    pub value: DVec3,
    /// Time range (for interface compatibility).
    pub times: Vec<f64>,
}

impl ConstantSpline {
    /// Create a new constant spline.
    pub fn new(value: DVec3) -> Self {
        Self {
            value,
            times: vec![0.0, 1.0],
        }
    }

    /// Create with a specific time range.
    pub fn with_time_range(value: DVec3, start: f64, end: f64) -> Self {
        Self {
            value,
            times: vec![start, end],
        }
    }

    /// Evaluate (always returns the constant value).
    pub fn evaluate(&self, _time: f64) -> DVec3 {
        self.value
    }
}

impl Spline for ConstantSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// MorphWeightSpline
// ============================================================================

/// Spline for morph target weights (scalar values).
///
/// Maps to CesiumJS `Core/MorphWeightSpline.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct MorphWeightSpline {
    /// Time values.
    pub times: Vec<f64>,
    /// Weight values (0.0 to 1.0 typically).
    pub weights: Vec<f64>,
}

impl MorphWeightSpline {
    /// Create a new morph weight spline.
    pub fn new(times: Vec<f64>, weights: Vec<f64>) -> Self {
        assert!(weights.len() >= 2, "weights.length must be >= 2");
        assert_eq!(times.len(), weights.len(), "times and weights must match");
        Self { times, weights }
    }

    /// Evaluate the weight at a given time (linear interpolation).
    pub fn evaluate(&self, time: f64) -> f64 {
        let i = self.find_time_interval(time);
        let t0 = self.times[i];
        let t1 = self.times[i + 1];
        let u = if (t1 - t0).abs() > 1e-15 {
            (time - t0) / (t1 - t0)
        } else {
            0.0
        };
        self.weights[i] + (self.weights[i + 1] - self.weights[i]) * u
    }
}

impl Spline for MorphWeightSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// ScalarSpline
// ============================================================================

/// Spline for scalar values (linear interpolation).
#[derive(Debug, Clone, PartialEq)]
pub struct ScalarSpline {
    /// Time values.
    pub times: Vec<f64>,
    /// Scalar values.
    pub values: Vec<f64>,
}

impl ScalarSpline {
    /// Create a new scalar spline.
    pub fn new(times: Vec<f64>, values: Vec<f64>) -> Self {
        assert!(values.len() >= 2, "values.length must be >= 2");
        assert_eq!(times.len(), values.len(), "times and values must match");
        Self { times, values }
    }

    /// Evaluate the scalar at a given time.
    pub fn evaluate(&self, time: f64) -> f64 {
        let i = self.find_time_interval(time);
        let t0 = self.times[i];
        let t1 = self.times[i + 1];
        let u = if (t1 - t0).abs() > 1e-15 {
            (time - t0) / (t1 - t0)
        } else {
            0.0
        };
        self.values[i] + (self.values[i + 1] - self.values[i]) * u
    }
}

impl Spline for ScalarSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    #[test]
    fn test_linear_spline_endpoints() {
        let spline = LinearSpline::new(
            vec![0.0, 1.0, 2.0],
            vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 1.0, 1.0),
                DVec3::new(2.0, 0.0, 0.0),
            ],
        );

        let p0 = spline.evaluate(0.0);
        assert!((p0 - DVec3::new(0.0, 0.0, 0.0)).length() < 1e-10);

        let p1 = spline.evaluate(1.0);
        assert!((p1 - DVec3::new(1.0, 1.0, 1.0)).length() < 1e-10);

        let p2 = spline.evaluate(2.0);
        assert!((p2 - DVec3::new(2.0, 0.0, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_linear_spline_midpoint() {
        let spline = LinearSpline::new(
            vec![0.0, 2.0],
            vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(4.0, 2.0, 0.0)],
        );

        let mid = spline.evaluate(1.0);
        assert!((mid - DVec3::new(2.0, 1.0, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_catmull_rom_spline_endpoints() {
        let spline = CatmullRomSpline::new(
            vec![0.0, 1.0, 2.0, 3.0],
            vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 1.0, 0.0),
                DVec3::new(2.0, 0.0, 0.0),
                DVec3::new(3.0, 1.0, 0.0),
            ],
        );

        let p0 = spline.evaluate(0.0);
        assert!((p0 - DVec3::new(0.0, 0.0, 0.0)).length() < 1e-10);

        let p3 = spline.evaluate(3.0);
        assert!((p3 - DVec3::new(3.0, 1.0, 0.0)).length() < 1e-6);
    }

    #[test]
    fn test_catmull_rom_smoothness() {
        let spline = CatmullRomSpline::new(
            vec![0.0, 1.0, 2.0],
            vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 1.0, 0.0),
                DVec3::new(2.0, 0.0, 0.0),
            ],
        );

        // Midpoint should be smooth (not exactly at control point)
        let mid = spline.evaluate(0.5);
        assert!(mid.x > 0.0 && mid.x < 1.0);
        assert!(mid.y > 0.0);
    }

    #[test]
    fn test_hermite_spline() {
        let spline = HermiteSpline::new(
            vec![0.0, 1.0],
            vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(1.0, 1.0, 0.0)],
            vec![DVec3::new(1.0, 0.0, 0.0), DVec3::new(1.0, 0.0, 0.0)],
            vec![DVec3::new(1.0, 1.0, 0.0), DVec3::new(1.0, 1.0, 0.0)],
        );

        let p0 = spline.evaluate(0.0);
        assert!((p0 - DVec3::new(0.0, 0.0, 0.0)).length() < 1e-10);

        let p1 = spline.evaluate(1.0);
        assert!((p1 - DVec3::new(1.0, 1.0, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_quaternion_spline() {
        let q0 = DQuat::IDENTITY;
        let q1 = DQuat::from_rotation_z(FRAC_PI_2);

        let spline = QuaternionSpline::new(vec![0.0, 1.0], vec![q0, q1]);

        let r0 = spline.evaluate(0.0);
        assert!((r0.x - q0.x).abs() < 1e-10);
        assert!((r0.w - q0.w).abs() < 1e-10);

        let r1 = spline.evaluate(1.0);
        assert!((r1.z - q1.z).abs() < 1e-6);
    }

    #[test]
    fn test_stepped_spline() {
        let spline = SteppedSpline::new(
            vec![0.0, 1.0, 2.0],
            vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 1.0, 1.0),
                DVec3::new(2.0, 2.0, 2.0),
            ],
        );

        // Should hold previous value
        let p = spline.evaluate(0.5);
        assert!((p - DVec3::new(0.0, 0.0, 0.0)).length() < 1e-10);

        let p = spline.evaluate(1.5);
        assert!((p - DVec3::new(1.0, 1.0, 1.0)).length() < 1e-10);
    }

    #[test]
    fn test_constant_spline() {
        let spline = ConstantSpline::new(DVec3::new(5.0, 5.0, 5.0));

        assert!((spline.evaluate(0.0) - DVec3::new(5.0, 5.0, 5.0)).length() < 1e-10);
        assert!((spline.evaluate(100.0) - DVec3::new(5.0, 5.0, 5.0)).length() < 1e-10);
    }

    #[test]
    fn test_morph_weight_spline() {
        let spline = MorphWeightSpline::new(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 0.5]);

        assert!((spline.evaluate(0.0) - 0.0).abs() < 1e-10);
        assert!((spline.evaluate(0.5) - 0.5).abs() < 1e-10);
        assert!((spline.evaluate(1.0) - 1.0).abs() < 1e-10);
        assert!((spline.evaluate(1.5) - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_scalar_spline() {
        let spline = ScalarSpline::new(vec![0.0, 1.0], vec![10.0, 20.0]);

        assert!((spline.evaluate(0.0) - 10.0).abs() < 1e-10);
        assert!((spline.evaluate(0.5) - 15.0).abs() < 1e-10);
        assert!((spline.evaluate(1.0) - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_wrap_time() {
        let spline = LinearSpline::new(
            vec![0.0, 1.0, 2.0],
            vec![DVec3::ZERO, DVec3::ONE, DVec3::ZERO],
        );

        assert!((spline.wrap_time(2.5) - 0.5).abs() < 1e-10);
        assert!((spline.wrap_time(-0.5) - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_clamp_time() {
        let spline = LinearSpline::new(
            vec![0.0, 1.0, 2.0],
            vec![DVec3::ZERO, DVec3::ONE, DVec3::ZERO],
        );

        assert!((spline.clamp_time(-1.0) - 0.0).abs() < 1e-10);
        assert!((spline.clamp_time(5.0) - 2.0).abs() < 1e-10);
        assert!((spline.clamp_time(1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_find_time_interval() {
        let spline = LinearSpline::new(
            vec![0.0, 1.0, 2.0, 3.0],
            vec![DVec3::ZERO, DVec3::ONE, DVec3::ZERO, DVec3::ONE],
        );

        assert_eq!(spline.find_time_interval(0.5), 0);
        assert_eq!(spline.find_time_interval(1.5), 1);
        assert_eq!(spline.find_time_interval(2.5), 2);
        assert_eq!(spline.find_time_interval(0.0), 0);
        assert_eq!(spline.find_time_interval(3.0), 2);
    }
}
