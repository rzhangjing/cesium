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
    /// Create a new Catmull-Rom spline with auto-computed tangents.
    ///
    /// Maps to CesiumJS CatmullRomSpline constructor (without firstTangent/lastTangent).
    pub fn new(times: Vec<f64>, points: Vec<DVec3>) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");

        let n = points.len();
        let (first_tangent, last_tangent) = if n > 2 {
            // CesiumJS: firstTangent = (2*points[1] - points[2] - points[0]) * 0.5
            let ft = (points[1] * 2.0 - points[2] - points[0]) * 0.5;
            // CesiumJS: lastTangent = (points[n-1] - 2*points[n-2] + points[n-3]) * 0.5
            let lt = (points[n - 1] - points[n - 2] * 2.0 + points[n - 3]) * 0.5;
            (ft, lt)
        } else {
            (points[1] - points[0], points[1] - points[0])
        };

        Self {
            times,
            points,
            first_tangent,
            last_tangent,
        }
    }

    /// Create with explicit tangents.
    ///
    /// Maps to CesiumJS CatmullRomSpline constructor with firstTangent/lastTangent.
    pub fn with_tangents(
        times: Vec<f64>,
        points: Vec<DVec3>,
        first_tangent: DVec3,
        last_tangent: DVec3,
    ) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");
        Self {
            times,
            points,
            first_tangent,
            last_tangent,
        }
    }

    /// Evaluate the spline at a given time.
    ///
    /// Uses Hermite basis for first/last segments, Catmull-Rom matrix for interior.
    pub fn evaluate(&self, time: f64) -> DVec3 {
        let n = self.points.len();
        if n < 3 {
            // Fall back to linear
            let t0 = self.times[0];
            let inv_span = 1.0 / (self.times[1] - t0);
            let u = (time - t0) * inv_span;
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

        let u2 = u * u;
        let u3 = u2 * u;

        if i == 0 {
            // First segment: Hermite with firstTangent
            let p0 = self.points[0];
            let p1 = self.points[1];
            let m0 = self.first_tangent;
            let m1 = (self.points[2] - self.points[0]) * 0.5;

            let h00 = 2.0 * u3 - 3.0 * u2 + 1.0;
            let h10 = u3 - 2.0 * u2 + u;
            let h01 = -2.0 * u3 + 3.0 * u2;
            let h11 = u3 - u2;

            p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11
        } else if i == n - 2 {
            // Last segment: Hermite with lastTangent
            let p0 = self.points[i];
            let p1 = self.points[i + 1];
            let m0 = (self.points[i + 1] - self.points[i - 1]) * 0.5;
            let m1 = self.last_tangent;

            let h00 = 2.0 * u3 - 3.0 * u2 + 1.0;
            let h10 = u3 - 2.0 * u2 + u;
            let h01 = -2.0 * u3 + 3.0 * u2;
            let h11 = u3 - u2;

            p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11
        } else {
            // Interior: Catmull-Rom coefficient matrix
            // Matrix: [-0.5, 1.5, -1.5, 0.5; 1.0, -2.5, 2.0, -0.5; -0.5, 0.0, 0.5, 0.0; 0.0, 1.0, 0.0, 0.0]
            let p0 = self.points[i - 1];
            let p1 = self.points[i];
            let p2 = self.points[i + 1];
            let p3 = self.points[i + 2];

            let c0 = -0.5 * u3 + u2 - 0.5 * u;
            let c1 = 1.5 * u3 - 2.5 * u2 + 1.0;
            let c2 = -1.5 * u3 + 2.0 * u2 + 0.5 * u;
            let c3 = 0.5 * u3 - 0.5 * u2;

            p0 * c0 + p1 * c1 + p2 * c2 + p3 * c3
        }
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
/// inTangents and outTangents have length `points.len() - 1`.
/// For segment [i, i+1]: out_tangents[i] is outgoing tangent at points[i],
/// in_tangents[i] is incoming tangent at points[i+1].
#[derive(Debug, Clone, PartialEq)]
pub struct HermiteSpline {
    /// Time values.
    pub times: Vec<f64>,
    /// Control points.
    pub points: Vec<DVec3>,
    /// Incoming tangents (length = points.len() - 1).
    pub in_tangents: Vec<DVec3>,
    /// Outgoing tangents (length = points.len() - 1).
    pub out_tangents: Vec<DVec3>,
}

impl HermiteSpline {
    /// Create a new Hermite spline.
    /// in_tangents and out_tangents must have length == points.len() - 1.
    pub fn new(
        times: Vec<f64>,
        points: Vec<DVec3>,
        in_tangents: Vec<DVec3>,
        out_tangents: Vec<DVec3>,
    ) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");
        assert_eq!(
            in_tangents.len(),
            points.len() - 1,
            "inTangents.length must be points.length - 1"
        );
        assert_eq!(
            out_tangents.len(),
            points.len() - 1,
            "outTangents.length must be points.length - 1"
        );
        Self {
            times,
            points,
            in_tangents,
            out_tangents,
        }
    }

    /// Creates a C1-continuous spline from shared tangents at each point.
    /// Maps to CesiumJS `HermiteSpline.createC1`.
    pub fn create_c1(times: Vec<f64>, points: Vec<DVec3>, tangents: Vec<DVec3>) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");
        assert_eq!(tangents.len(), points.len(), "tangents and points must match");
        let out_tangents = tangents[..tangents.len() - 1].to_vec();
        let in_tangents = tangents[1..].to_vec();
        Self { times, points, in_tangents, out_tangents }
    }

    /// Creates a natural cubic spline (C2 continuous).
    /// Maps to CesiumJS `HermiteSpline.createNaturalCubic`.
    pub fn create_natural_cubic(times: Vec<f64>, points: Vec<DVec3>) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");

        if points.len() < 3 {
            let tangent = points[1] - points[0];
            return Self {
                times,
                points,
                in_tangents: vec![tangent],
                out_tangents: vec![tangent],
            };
        }

        let tangents = generate_natural(&points);
        let out_tangents = tangents[..tangents.len() - 1].to_vec();
        let in_tangents = tangents[1..].to_vec();
        Self { times, points, in_tangents, out_tangents }
    }

    /// Creates a clamped cubic spline (C2 with specified endpoint tangents).
    /// Maps to CesiumJS `HermiteSpline.createClampedCubic`.
    pub fn create_clamped_cubic(
        times: Vec<f64>,
        points: Vec<DVec3>,
        first_tangent: DVec3,
        last_tangent: DVec3,
    ) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");

        if points.len() < 3 {
            let tangent = points[1] - points[0];
            return Self {
                times,
                points,
                in_tangents: vec![tangent],
                out_tangents: vec![tangent],
            };
        }

        let tangents = generate_clamped(&points, first_tangent, last_tangent);
        let out_tangents = tangents[..tangents.len() - 1].to_vec();
        let in_tangents = tangents[1..].to_vec();
        Self { times, points, in_tangents, out_tangents }
    }

    /// Evaluate the spline at a given time.
    /// Uses CesiumJS hermite coefficient matrix with timesDelta scaling.
    pub fn evaluate(&self, time: f64) -> DVec3 {
        let i = self.find_time_interval(time);
        let t0 = self.times[i];
        let t1 = self.times[i + 1];
        let times_delta = t1 - t0;
        let u = if times_delta.abs() > 1e-15 {
            (time - t0) / times_delta
        } else {
            0.0
        };

        let u2 = u * u;
        let u3 = u2 * u;
        // Hermite basis from hermiteCoefficientMatrix, tangent coefs scaled by timesDelta
        let coef_start = 2.0 * u3 - 3.0 * u2 + 1.0;
        let coef_end = -2.0 * u3 + 3.0 * u2;
        let coef_out = (u3 - 2.0 * u2 + u) * times_delta;
        let coef_in = (u3 - u2) * times_delta;

        self.points[i] * coef_start
            + self.points[i + 1] * coef_end
            + self.out_tangents[i] * coef_out
            + self.in_tangents[i] * coef_in
    }
}

impl Spline for HermiteSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

/// Solves a tridiagonal system using the Thomas Algorithm.
/// Maps to CesiumJS `TridiagonalSystemSolver.solve`.
pub fn tridiagonal_solve(
    lower: &[f64],
    diagonal: &[f64],
    upper: &[f64],
    right: &[DVec3],
) -> Vec<DVec3> {
    let n = right.len();
    let mut c = vec![0.0f64; upper.len()];
    let mut d = vec![DVec3::ZERO; n];
    let mut x = vec![DVec3::ZERO; n];

    c[0] = upper[0] / diagonal[0];
    d[0] = right[0] * (1.0 / diagonal[0]);

    for i in 1..c.len() {
        let scalar = 1.0 / (diagonal[i] - c[i - 1] * lower[i - 1]);
        c[i] = upper[i] * scalar;
        d[i] = (right[i] - d[i - 1] * lower[i - 1]) * scalar;
    }

    let i = c.len();
    let scalar = 1.0 / (diagonal[i] - c[i - 1] * lower[i - 1]);
    d[i] = (right[i] - d[i - 1] * lower[i - 1]) * scalar;

    x[n - 1] = d[n - 1];
    for i in (0..n - 1).rev() {
        x[i] = d[i] - x[i + 1] * c[i];
    }

    x
}

/// Generates tangents for a natural cubic spline.
/// Maps to CesiumJS `generateNatural`.
fn generate_natural(points: &[DVec3]) -> Vec<DVec3> {
    let n = points.len();
    let mut l = vec![0.0f64; n - 1];
    let mut u = vec![0.0f64; n - 1];
    let mut d = vec![0.0f64; n];
    let mut r = vec![DVec3::ZERO; n];

    l[0] = 1.0;
    u[0] = 1.0;
    d[0] = 2.0;
    r[0] = (points[1] - points[0]) * 3.0;

    for i in 1..n - 1 {
        l[i] = 1.0;
        u[i] = 1.0;
        d[i] = 4.0;
        r[i] = (points[i + 1] - points[i - 1]) * 3.0;
    }

    d[n - 1] = 2.0;
    r[n - 1] = (points[n - 1] - points[n - 2]) * 3.0;

    tridiagonal_solve(&l, &d, &u, &r)
}

/// Generates tangents for a clamped cubic spline.
/// Maps to CesiumJS `generateClamped`.
fn generate_clamped(points: &[DVec3], first_tangent: DVec3, last_tangent: DVec3) -> Vec<DVec3> {
    let n = points.len();
    let mut l = vec![0.0f64; n - 1];
    let mut u = vec![0.0f64; n - 1];
    let mut d = vec![0.0f64; n];
    let mut r = vec![DVec3::ZERO; n];

    l[0] = 1.0;
    d[0] = 1.0;
    u[0] = 0.0;
    r[0] = first_tangent;

    for i in 1..n - 2 {
        l[i] = 1.0;
        u[i] = 1.0;
        d[i] = 4.0;
        r[i] = (points[i + 1] - points[i - 1]) * 3.0;
    }

    let i = n - 2;
    l[i] = 0.0;
    u[i] = 1.0;
    d[i] = 4.0;
    r[i] = (points[i + 1] - points[i - 1]) * 3.0;

    d[n - 1] = 1.0;
    r[n - 1] = last_tangent;

    tridiagonal_solve(&l, &d, &u, &r)
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
/// Maps to CesiumJS `Core/SteppedSpline.js`.
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

    /// wrapTime always returns 0.0 for a constant spline.
    pub fn wrap_time(&self, _time: f64) -> f64 {
        0.0
    }

    /// clampTime always returns 0.0 for a constant spline.
    pub fn clamp_time(&self, _time: f64) -> f64 {
        0.0
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
    fn test_hermite_spline() {
        let spline = HermiteSpline::new(
            vec![0.0, 1.0],
            vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(1.0, 1.0, 0.0)],
            vec![DVec3::new(1.0, 0.0, 0.0)],
            vec![DVec3::new(1.0, 0.0, 0.0)],
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
        let p = spline.evaluate(0.5);
        assert!((p - DVec3::new(0.0, 0.0, 0.0)).length() < 1e-10);
        let p = spline.evaluate(1.5);
        assert!((p - DVec3::new(1.0, 1.0, 1.0)).length() < 1e-10);
    }

    #[test]
    fn test_natural_cubic() {
        let spline = HermiteSpline::create_natural_cubic(
            vec![0.0, 1.0, 2.0, 3.0],
            vec![
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.0, 1.0, FRAC_PI_2),
                DVec3::new(-1.0, 0.0, std::f64::consts::PI),
                DVec3::new(0.0, -1.0, 3.0 * FRAC_PI_2),
            ],
        );
        let p0 = spline.evaluate(0.0);
        assert!((p0 - DVec3::new(1.0, 0.0, 0.0)).length() < 1e-10);
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
