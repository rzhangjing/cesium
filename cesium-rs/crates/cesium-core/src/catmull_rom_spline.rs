//! Ported from `packages/engine/Source/Core/CatmullRomSpline.js`.

use crate::cartesian3::Cartesian3;
use crate::spline::{clamp_time, find_time_interval, wrap_time, SplinePoint};

/// A Catmull-Rom spline is a cubic spline where the tangent at control
/// points, except the first and last, are computed using the previous and
/// next control points. Catmull-Rom splines are in the class C¹.
pub struct CatmullRomSpline {
    times: Vec<f64>,
    points: Vec<SplinePoint>,
    first_tangent: Option<SplinePoint>,
    last_tangent: Option<SplinePoint>,
    last_time_index: usize,
}

impl CatmullRomSpline {
    /// Creates a new CatmullRomSpline without explicit boundary tangents;
    /// they are estimated when there are more than two control points
    /// (mirrors `new CatmullRomSpline({ times, points })`).
    pub fn new(times: Vec<f64>, points: Vec<SplinePoint>) -> Self {
        Self::new_with_tangents(times, points, None, None)
    }

    /// Creates a new CatmullRomSpline with optional `firstTangent` /
    /// `lastTangent` (mirrors the full JS options object).
    pub fn new_with_tangents(
        times: Vec<f64>,
        points: Vec<SplinePoint>,
        first_tangent: Option<SplinePoint>,
        last_tangent: Option<SplinePoint>,
    ) -> Self {
        //>>includeStart('debug', pragmas.debug) equivalent:
        debug_assert!(
            points.len() >= 2,
            "points.length must be greater than or equal to 2."
        );
        debug_assert!(
            times.len() == points.len(),
            "times.length must be equal to points.length."
        );
        //>>includeEnd('debug')

        let mut first_tangent = first_tangent;
        let mut last_tangent = last_tangent;

        if points.len() > 2 {
            if first_tangent.is_none() {
                // firstTangent = 0.5 * (2*points[1] - points[2] - points[0])
                let mut t = spline_point_scale(&points[1], 2.0);
                t = spline_point_sub(&t, &points[2]);
                t = spline_point_sub(&t, &points[0]);
                first_tangent = Some(spline_point_scale(&t, 0.5));
            }

            if last_tangent.is_none() {
                // lastTangent = 0.5 * (points[n] - 2*points[n-1] + points[n-2])
                let n = points.len() - 1;
                let doubled = spline_point_scale(&points[n - 1], 2.0);
                let mut t = spline_point_sub(&points[n], &doubled);
                t = spline_point_add(&t, &points[n - 2]);
                last_tangent = Some(spline_point_scale(&t, 0.5));
            }
        }

        Self {
            times,
            points,
            first_tangent,
            last_tangent,
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

    /// The tangent at the first control point (`None` when constructed with
    /// fewer than three control points, like the JS `clone(undefined)`).
    pub fn first_tangent(&self) -> Option<&SplinePoint> {
        self.first_tangent.as_ref()
    }

    /// The tangent at the last control point.
    pub fn last_tangent(&self) -> Option<&SplinePoint> {
        self.last_tangent.as_ref()
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
    ///
    /// Mirrors `createEvaluateFunction`: with fewer than three control
    /// points the spline degrades to a straight lerp over the single span
    /// (no range check, exactly like the JS closure); otherwise the first
    /// and last segments blend with the (estimated or given) boundary
    /// tangents through the Hermite coefficient matrix, while interior
    /// segments use the Catmull-Rom coefficient matrix.
    pub fn evaluate(&mut self, time: f64) -> Option<SplinePoint> {
        if self.points.len() < 3 {
            let t0 = self.times[0];
            let inv_span = 1.0 / (self.times[1] - t0);
            let u = (time - t0) * inv_span;
            return Some(SplinePoint::lerp(&self.points[0], &self.points[1], u));
        }

        let i = find_time_interval(&self.times, time, Some(self.last_time_index))?;
        self.last_time_index = i;
        let u = (time - self.times[i]) / (self.times[i + 1] - self.times[i]);

        let u2 = u * u;
        let u3 = u2 * u;

        let (p0, p1, p2, p3, coefs);

        if i == 0 {
            p0 = self.points[0].clone_point();
            p1 = self.points[1].clone_point();
            p2 = self.first_tangent.clone().expect("first_tangent estimated");

            // p3 = 0.5 * (points[2] - points[0])
            let diff = spline_point_sub(&self.points[2], &self.points[0]);
            p3 = spline_point_scale(&diff, 0.5);

            coefs = hermite_coefficients(u3, u2, u);
        } else if i == self.points.len() - 2 {
            p0 = self.points[i].clone_point();
            p1 = self.points[i + 1].clone_point();
            p3 = self.last_tangent.clone().expect("last_tangent estimated");

            // p2 = 0.5 * (points[i + 1] - points[i - 1])
            let diff = spline_point_sub(&self.points[i + 1], &self.points[i - 1]);
            p2 = spline_point_scale(&diff, 0.5);

            coefs = hermite_coefficients(u3, u2, u);
        } else {
            p0 = self.points[i - 1].clone_point();
            p1 = self.points[i].clone_point();
            p2 = self.points[i + 1].clone_point();
            p3 = self.points[i + 2].clone_point();

            coefs = catmull_rom_coefficients(u3, u2, u);
        }

        Some(spline_point_combine(&p0, coefs.0, &p1, coefs.1, &p2, coefs.2, &p3, coefs.3))
    }
}

/// `HermiteSpline.hermiteCoefficientMatrix` applied to `[u³, u², u, 1]`
/// via `Matrix4.multiplyByVector`; coefficient order is (start point, end
/// point, out-tangent, in-tangent).
fn hermite_coefficients(u3: f64, u2: f64, u: f64) -> (f64, f64, f64, f64) {
    (
        2.0 * u3 - 3.0 * u2 + 1.0,
        -2.0 * u3 + 3.0 * u2,
        u3 - 2.0 * u2 + u,
        u3 - u2,
    )
}

/// `CatmullRomSpline.catmullRomCoefficientMatrix` applied to
/// `[u³, u², u, 1]` via `Matrix4.multiplyByVector` (coefficient rows read
/// as `m[0],m[4],m[8],m[12]` etc.); order is (p0, p1, p2, p3).
fn catmull_rom_coefficients(u3: f64, u2: f64, u: f64) -> (f64, f64, f64, f64) {
    (
        -0.5 * u3 + u2 - 0.5 * u,
        1.5 * u3 - 2.5 * u2 + 1.0,
        -1.5 * u3 + 2.0 * u2 + 0.5 * u,
        0.5 * u3 - 0.5 * u2,
    )
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

fn spline_point_add(a: &SplinePoint, b: &SplinePoint) -> SplinePoint {
    match (a, b) {
        (SplinePoint::Scalar(va), SplinePoint::Scalar(vb)) => SplinePoint::Scalar(va + vb),
        (SplinePoint::Cartesian3(va), SplinePoint::Cartesian3(vb)) => {
            let mut result = Cartesian3::ZERO;
            Cartesian3::add(va, vb, &mut result);
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

/// result = p0*c0 + p1*c1 + p2*c2 + p3*c3 (mirrors the JS multiplyByScalar/
/// add chain at the end of the evaluate closure).
fn spline_point_combine(
    p0: &SplinePoint,
    c0: f64,
    p1: &SplinePoint,
    c1: f64,
    p2: &SplinePoint,
    c2: f64,
    p3: &SplinePoint,
    c3: f64,
) -> SplinePoint {
    match (p0, p1, p2, p3) {
        (
            SplinePoint::Scalar(v0),
            SplinePoint::Scalar(v1),
            SplinePoint::Scalar(v2),
            SplinePoint::Scalar(v3),
        ) => SplinePoint::Scalar(c0 * v0 + c1 * v1 + c2 * v2 + c3 * v3),
        (
            SplinePoint::Cartesian3(v0),
            SplinePoint::Cartesian3(v1),
            SplinePoint::Cartesian3(v2),
            SplinePoint::Cartesian3(v3),
        ) => {
            let mut tmp = Cartesian3::ZERO;
            let mut out = Cartesian3::ZERO;
            Cartesian3::multiply_by_scalar(v0, c0, &mut out);
            let mut acc = out;
            Cartesian3::multiply_by_scalar(v1, c1, &mut tmp);
            Cartesian3::add(&acc, &tmp, &mut out);
            acc = out;
            Cartesian3::multiply_by_scalar(v2, c2, &mut tmp);
            Cartesian3::add(&acc, &tmp, &mut out);
            acc = out;
            Cartesian3::multiply_by_scalar(v3, c3, &mut tmp);
            Cartesian3::add(&acc, &tmp, &mut out);
            SplinePoint::Cartesian3(out)
        }
        _ => p0.clone(),
    }
}
