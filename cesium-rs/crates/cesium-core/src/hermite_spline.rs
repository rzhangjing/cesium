//! Ported from `packages/engine/Source/Core/HermiteSpline.js`.

use crate::cartesian3::Cartesian3;
use crate::spline::{clamp_time, find_time_interval, wrap_time, SplinePoint};

/// A spline that uses piecewise Hermite interpolation to create a curve.
pub struct HermiteSpline {
    times: Vec<f64>,
    points: Vec<SplinePoint>,
    in_tangents: Vec<SplinePoint>,
    out_tangents: Vec<SplinePoint>,
    last_time_index: usize,
}

impl HermiteSpline {
    /// Creates a new HermiteSpline.
    pub fn new(
        times: Vec<f64>,
        points: Vec<SplinePoint>,
        in_tangents: Vec<SplinePoint>,
        out_tangents: Vec<SplinePoint>,
    ) -> Self {
        Self {
            times,
            points,
            in_tangents,
            out_tangents,
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

    /// Evaluates the curve at a given time using Hermite basis functions.
    pub fn evaluate(&mut self, time: f64) -> Option<SplinePoint> {
        let i = find_time_interval(&self.times, time, Some(self.last_time_index))?;
        self.last_time_index = i;

        let dt = self.times[i + 1] - self.times[i];
        let t = (time - self.times[i]) / dt;

        let t2 = t * t;
        let t3 = t2 * t;

        // Hermite basis functions
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        let p0 = &self.points[i];
        let p1 = &self.points[i + 1];
        let m0 = &self.out_tangents[i];
        let m1 = &self.in_tangents[i + 1];

        Some(hermite_combine(p0, m0, p1, m1, h00, h10 * dt, h01, h11 * dt))
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
            let mut result = Cartesian3::ZERO;
            let mut tmp = Cartesian3::ZERO;

            Cartesian3::multiply_by_scalar(p0v, h00, &mut tmp);
            let mut acc = tmp;
            Cartesian3::multiply_by_scalar(m0v, h10, &mut tmp);
            Cartesian3::add(&acc, &tmp, &mut result);
            acc = result;
            Cartesian3::multiply_by_scalar(p1v, h01, &mut tmp);
            Cartesian3::add(&acc, &tmp, &mut result);
            acc = result;
            Cartesian3::multiply_by_scalar(m1v, h11, &mut tmp);
            Cartesian3::add(&acc, &tmp, &mut result);

            SplinePoint::Cartesian3(result)
        }
        _ => p0.clone(),
    }
}
