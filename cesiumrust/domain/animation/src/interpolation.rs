//! Time interpolation algorithms for animation.
//!
//! Maps to CesiumJS interpolation:
//! - `Core/HermitePolynomialApproximation.js`
//! - `Core/LagrangePolynomialApproximation.js`
//! - `Core/LinearApproximation.js`
//! - `Core/InterpolationAlgorithm.js`

use glam::DVec3;

/// Interpolation algorithm type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InterpolationType {
    /// Linear interpolation (degree 1).
    #[default]
    Linear,
    /// Hermite polynomial interpolation (uses derivatives).
    Hermite,
    /// Lagrange polynomial interpolation.
    Lagrange,
}

/// A time-value sample point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplePoint {
    /// Time in seconds from epoch.
    pub time: f64,
    /// Value at this time.
    pub value: f64,
    /// Optional derivative (for Hermite).
    pub derivative: Option<f64>,
}

impl SamplePoint {
    /// Creates a new sample point.
    pub fn new(time: f64, value: f64) -> Self {
        Self {
            time,
            value,
            derivative: None,
        }
    }

    /// Creates a sample point with derivative.
    pub fn with_derivative(time: f64, value: f64, derivative: f64) -> Self {
        Self {
            time,
            value,
            derivative: Some(derivative),
        }
    }
}

/// Linear interpolation between two values.
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Linear interpolation for DVec3.
pub fn lerp_vec3(a: DVec3, b: DVec3, t: f64) -> DVec3 {
    a + (b - a) * t
}

/// Hermite interpolation (cubic) between two points with tangents.
///
/// # Arguments
/// * `p0` - Start value
/// * `m0` - Start tangent
/// * `p1` - End value
/// * `m1` - End tangent
/// * `t` - Parameter [0, 1]
pub fn hermite(p0: f64, m0: f64, p1: f64, m1: f64, t: f64) -> f64 {
    let t2 = t * t;
    let t3 = t2 * t;

    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;

    h00 * p0 + h10 * m0 + h01 * p1 + h11 * m1
}

/// Hermite interpolation for DVec3.
pub fn hermite_vec3(p0: DVec3, m0: DVec3, p1: DVec3, m1: DVec3, t: f64) -> DVec3 {
    DVec3::new(
        hermite(p0.x, m0.x, p1.x, m1.x, t),
        hermite(p0.y, m0.y, p1.y, m1.y, t),
        hermite(p0.z, m0.z, p1.z, m1.z, t),
    )
}

/// Lagrange polynomial interpolation.
///
/// # Arguments
/// * `points` - Sample points (time, value)
/// * `t` - Time to interpolate at
pub fn lagrange_interpolate(points: &[SamplePoint], t: f64) -> f64 {
    let n = points.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return points[0].value;
    }

    let mut result = 0.0;
    for i in 0..n {
        let mut basis = points[i].value;
        for j in 0..n {
            if i != j {
                let denom = points[i].time - points[j].time;
                if denom.abs() > 1e-15 {
                    basis *= (t - points[j].time) / denom;
                }
            }
        }
        result += basis;
    }
    result
}

/// Lagrange interpolation for DVec3.
pub fn lagrange_interpolate_vec3(
    times: &[f64],
    values: &[DVec3],
    t: f64,
) -> DVec3 {
    let n = times.len().min(values.len());
    if n == 0 {
        return DVec3::ZERO;
    }

    let points_x: Vec<SamplePoint> = (0..n)
        .map(|i| SamplePoint::new(times[i], values[i].x))
        .collect();
    let points_y: Vec<SamplePoint> = (0..n)
        .map(|i| SamplePoint::new(times[i], values[i].y))
        .collect();
    let points_z: Vec<SamplePoint> = (0..n)
        .map(|i| SamplePoint::new(times[i], values[i].z))
        .collect();

    DVec3::new(
        lagrange_interpolate(&points_x, t),
        lagrange_interpolate(&points_y, t),
        lagrange_interpolate(&points_z, t),
    )
}

/// Catmull-Rom spline interpolation (a type of Hermite with auto-tangents).
///
/// # Arguments
/// * `p0`, `p1`, `p2`, `p3` - Four control points
/// * `t` - Parameter [0, 1] (interpolates between p1 and p2)
pub fn catmull_rom(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    let t2 = t * t;
    let t3 = t2 * t;

    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// Catmull-Rom spline for DVec3.
pub fn catmull_rom_vec3(p0: DVec3, p1: DVec3, p2: DVec3, p3: DVec3, t: f64) -> DVec3 {
    DVec3::new(
        catmull_rom(p0.x, p1.x, p2.x, p3.x, t),
        catmull_rom(p0.y, p1.y, p2.y, p3.y, t),
        catmull_rom(p0.z, p1.z, p2.z, p3.z, t),
    )
}

/// Spherical linear interpolation for unit vectors (directions).
pub fn slerp_vec3(a: DVec3, b: DVec3, t: f64) -> DVec3 {
    let dot = a.dot(b).clamp(-1.0, 1.0);

    if dot.abs() > 0.9995 {
        // Nearly parallel, use linear interpolation
        return lerp_vec3(a, b, t).normalize();
    }

    let theta = dot.acos();
    let sin_theta = theta.sin();
    let wa = ((1.0 - t) * theta).sin() / sin_theta;
    let wb = (t * theta).sin() / sin_theta;

    (a * wa + b * wb).normalize()
}

/// Interpolates a value using the specified algorithm.
pub fn interpolate(
    algo: InterpolationType,
    points: &[SamplePoint],
    t: f64,
) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    if points.len() == 1 {
        return points[0].value;
    }

    match algo {
        InterpolationType::Linear => {
            // Find bracketing interval
            let (i0, i1) = find_bracket(points, t);
            let dt = points[i1].time - points[i0].time;
            let frac = if dt.abs() > 1e-15 {
                (t - points[i0].time) / dt
            } else {
                0.0
            };
            lerp(points[i0].value, points[i1].value, frac)
        }
        InterpolationType::Hermite => {
            let (i0, i1) = find_bracket(points, t);
            let dt = points[i1].time - points[i0].time;
            let frac = if dt.abs() > 1e-15 {
                (t - points[i0].time) / dt
            } else {
                0.0
            };
            let m0 = points[i0].derivative.unwrap_or(0.0) * dt;
            let m1 = points[i1].derivative.unwrap_or(0.0) * dt;
            hermite(points[i0].value, m0, points[i1].value, m1, frac)
        }
        InterpolationType::Lagrange => lagrange_interpolate(points, t),
    }
}

/// Finds the bracketing indices for time t.
fn find_bracket(points: &[SamplePoint], t: f64) -> (usize, usize) {
    if t <= points[0].time {
        return (0, 1.min(points.len() - 1));
    }
    let last = points.len() - 1;
    if t >= points[last].time {
        return (last.saturating_sub(1), last);
    }

    for i in 0..last {
        if t >= points[i].time && t <= points[i + 1].time {
            return (i, i + 1);
        }
    }
    (last.saturating_sub(1), last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0, 10.0, 0.0) - 0.0).abs() < 1e-10);
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < 1e-10);
        assert!((lerp(0.0, 10.0, 1.0) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_lerp_vec3() {
        let a = DVec3::new(0.0, 0.0, 0.0);
        let b = DVec3::new(10.0, 20.0, 30.0);
        let result = lerp_vec3(a, b, 0.5);
        assert!((result.x - 5.0).abs() < 1e-10);
        assert!((result.y - 10.0).abs() < 1e-10);
        assert!((result.z - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_hermite_endpoints() {
        // Hermite should pass through endpoints
        let result_start = hermite(1.0, 0.0, 5.0, 0.0, 0.0);
        let result_end = hermite(1.0, 0.0, 5.0, 0.0, 1.0);
        assert!((result_start - 1.0).abs() < 1e-10);
        assert!((result_end - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_hermite_midpoint() {
        // With zero tangents, midpoint should be average
        let result = hermite(0.0, 0.0, 10.0, 0.0, 0.5);
        assert!((result - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_lagrange_linear() {
        // Two points = linear interpolation
        let points = vec![SamplePoint::new(0.0, 0.0), SamplePoint::new(1.0, 10.0)];
        let result = lagrange_interpolate(&points, 0.5);
        assert!((result - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_lagrange_quadratic() {
        // Three points on y = x^2
        let points = vec![
            SamplePoint::new(0.0, 0.0),
            SamplePoint::new(1.0, 1.0),
            SamplePoint::new(2.0, 4.0),
        ];
        let result = lagrange_interpolate(&points, 1.5);
        assert!((result - 2.25).abs() < 1e-10);
    }

    #[test]
    fn test_catmull_rom_endpoints() {
        let result_start = catmull_rom(0.0, 1.0, 4.0, 9.0, 0.0);
        let result_end = catmull_rom(0.0, 1.0, 4.0, 9.0, 1.0);
        assert!((result_start - 1.0).abs() < 1e-10);
        assert!((result_end - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_slerp_same_direction() {
        let a = DVec3::X;
        let b = DVec3::X;
        let result = slerp_vec3(a, b, 0.5);
        assert!((result.x - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_slerp_perpendicular() {
        let a = DVec3::X;
        let b = DVec3::Y;
        let result = slerp_vec3(a, b, 0.5);
        // Should be at 45 degrees
        let expected = (std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2);
        assert!((result.x - expected.0).abs() < 1e-10);
        assert!((result.y - expected.1).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_linear() {
        let points = vec![
            SamplePoint::new(0.0, 0.0),
            SamplePoint::new(10.0, 100.0),
        ];
        let result = interpolate(InterpolationType::Linear, &points, 5.0);
        assert!((result - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_hermite() {
        let points = vec![
            SamplePoint::with_derivative(0.0, 0.0, 0.0),
            SamplePoint::with_derivative(1.0, 1.0, 0.0),
        ];
        let result = interpolate(InterpolationType::Hermite, &points, 0.5);
        assert!((result - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_lagrange() {
        let points = vec![
            SamplePoint::new(0.0, 0.0),
            SamplePoint::new(1.0, 1.0),
            SamplePoint::new(2.0, 4.0),
        ];
        let result = interpolate(InterpolationType::Lagrange, &points, 0.5);
        assert!((result - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_empty() {
        let result = interpolate(InterpolationType::Linear, &[], 0.5);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_interpolate_single() {
        let points = vec![SamplePoint::new(0.0, 42.0)];
        let result = interpolate(InterpolationType::Linear, &points, 0.5);
        assert_eq!(result, 42.0);
    }

    #[test]
    fn test_interpolation_type_default() {
        assert_eq!(InterpolationType::default(), InterpolationType::Linear);
    }
}
