//! Ported from `packages/engine/Source/Core/CatmullRomSpline.js`.

use crate::cartesian3::Cartesian3;
use crate::spline::{clamp_time, find_time_interval, wrap_time, SplinePoint};

/// A spline that uses piecewise Catmull-Rom interpolation to create a curve.
pub struct CatmullRomSpline {
    times: Vec<f64>,
    points: Vec<SplinePoint>,
    last_time_index: usize,
}

impl CatmullRomSpline {
    /// Creates a new CatmullRomSpline.
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

    /// Wraps time.
    pub fn wrap_time(&self, time: f64) -> f64 {
        wrap_time(&self.times, time)
    }

    /// Clamps time.
    pub fn clamp_time(&self, time: f64) -> f64 {
        clamp_time(&self.times, time)
    }

    /// Evaluates the curve at a given time.
    pub fn evaluate(&mut self, time: f64) -> Option<SplinePoint> {
        let i = find_time_interval(&self.times, time, Some(self.last_time_index))?;
        self.last_time_index = i;

        let dt = self.times[i + 1] - self.times[i];
        let t = (time - self.times[i]) / dt;

        // Catmull-Rom tangents
        let p0 = if i > 0 { &self.points[i - 1] } else { &self.points[i] };
        let p1 = &self.points[i];
        let p2 = &self.points[i + 1];
        let p3 = if i + 2 < self.points.len() {
            &self.points[i + 2]
        } else {
            &self.points[i + 1]
        };

        // Compute tangents using central differences
        let m0 = spline_point_sub(p2, p0);
        let m0 = spline_point_scale(&m0, 0.5);
        let m1 = spline_point_sub(p3, p1);
        let m1 = spline_point_scale(&m1, 0.5);

        let t2 = t * t;
        let t3 = t2 * t;

        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        Some(hermite_combine(p1, &m0, p2, &m1, h00, h10 * dt, h01, h11 * dt))
    }
}

fn spline_point_sub(a: &SplinePoint, b: &SplinePoint) -> SplinePoint {
    match (a, b) {
        (SplinePoint::Scalar(va), SplinePoint::Scalar(vb)) => SplinePoint::Scalar(va - vb),
        (SplinePoint::Cartesian3(va), SplinePoint::Cartesian3(vb)) => {
            let mut result = Cartesian3::ZERO;
            Cartesian3::subtract(va, vb, &mut result);
            SplinePoint::Cartesian3(result)
        }
        _ => a.clone(),
    }
}

fn spline_point_scale(a: &SplinePoint, s: f64) -> SplinePoint {
    match a {
        SplinePoint::Scalar(v) => SplinePoint::Scalar(v * s),
        SplinePoint::Cartesian3(v) => {
            let mut result = Cartesian3::ZERO;
            Cartesian3::multiply_by_scalar(v, s, &mut result);
            SplinePoint::Cartesian3(result)
        }
    }
}

fn hermite_combine(
    p0: &SplinePoint,
    m0: &SplinePoint,
    p1: &SplinePoint,
    m1: &SplinePoint,
    h00: f64,
    h10: f64,
    h01: f64,
    h11: f64,
) -> SplinePoint {
    match (p0, m0, p1, m1) {
        (
            SplinePoint::Scalar(p0v),
            SplinePoint::Scalar(m0v),
            SplinePoint::Scalar(p1v),
            SplinePoint::Scalar(m1v),
        ) => SplinePoint::Scalar(h00 * p0v + h10 * m0v + h01 * p1v + h11 * m1v),
        (
            SplinePoint::Cartesian3(p0v),
            SplinePoint::Cartesian3(m0v),
            SplinePoint::Cartesian3(p1v),
            SplinePoint::Cartesian3(m1v),
        ) => {
            let mut tmp = Cartesian3::ZERO;
            let mut acc;
            let mut out = Cartesian3::ZERO;
            Cartesian3::multiply_by_scalar(p0v, h00, &mut tmp);
            acc = tmp;
            Cartesian3::multiply_by_scalar(m0v, h10, &mut tmp);
            Cartesian3::add(&acc, &tmp, &mut out);
            acc = out;
            Cartesian3::multiply_by_scalar(p1v, h01, &mut tmp);
            Cartesian3::add(&acc, &tmp, &mut out);
            acc = out;
            Cartesian3::multiply_by_scalar(m1v, h11, &mut tmp);
            Cartesian3::add(&acc, &tmp, &mut out);
            SplinePoint::Cartesian3(out)
        }
        _ => p0.clone(),
    }
}
