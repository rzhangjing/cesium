//! Ported from `packages/engine/Source/Core/EllipsoidGeodesic.js`.
//!
//! Computes the geodesic path between two points on an ellipsoid using Vincenty's
//! formulae. The *inverse* problem (`vincenty_inverse_formula`) yields the surface
//! distance and the two headings; `set_constants` then captures the series
//! constants of the *direct* problem so `interpolate_using_surface_distance` can
//! walk back along the same geodesic.

use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::math::CesiumMath;

/// The `_constants` bag of CesiumJS: everything `setConstants` derives from the
/// start point plus the inverse solution, reused by every interpolation call.
#[derive(Debug, Clone, Copy, Default)]
struct GeodesicConstants {
    a: f64,
    b: f64,
    f: f64,
    cosine_heading: f64,
    sine_heading: f64,
    /// Stored by CesiumJS `setConstants` (`constants.tanU`) but never read back,
    /// exactly like `a0`..`a3` below.
    #[allow(dead_code)]
    tan_u: f64,
    cosine_u: f64,
    sine_u: f64,
    sigma: f64,
    sine_alpha: f64,
    #[allow(dead_code)]
    sine_squared_alpha: f64,
    cosine_squared_alpha: f64,
    cosine_alpha: f64,
    u2_over_4: f64,
    u4_over_16: f64,
    u6_over_64: f64,
    u8_over_256: f64,
    #[allow(dead_code)]
    a0: f64,
    #[allow(dead_code)]
    a1: f64,
    #[allow(dead_code)]
    a2: f64,
    #[allow(dead_code)]
    a3: f64,
    distance_ratio: f64,
}

/// `computeC` of CesiumJS.
fn compute_c(f: f64, cosine_squared_alpha: f64) -> f64 {
    (f * cosine_squared_alpha * (4.0 + f * (4.0 - 3.0 * cosine_squared_alpha))) / 16.0
}

/// `computeDeltaLambda` of CesiumJS.
#[allow(clippy::too_many_arguments)]
fn compute_delta_lambda(
    f: f64,
    sine_alpha: f64,
    cosine_squared_alpha: f64,
    sigma: f64,
    sine_sigma: f64,
    cosine_sigma: f64,
    cosine_twice_sigma_midpoint: f64,
) -> f64 {
    let c = compute_c(f, cosine_squared_alpha);

    (1.0 - c)
        * f
        * sine_alpha
        * (sigma
            + c * sine_sigma
                * (cosine_twice_sigma_midpoint
                    + c * cosine_sigma
                        * (2.0 * cosine_twice_sigma_midpoint * cosine_twice_sigma_midpoint - 1.0)))
}

/// The outputs `vincentyInverseFormula` writes back onto the geodesic.
struct InverseSolution {
    distance: f64,
    start_heading: f64,
    end_heading: f64,
    u_squared: f64,
}

/// `vincentyInverseFormula` of CesiumJS.
///
/// DEVIATION: the JS `do { ... } while (|lambda - lambdaDot| > EPSILON12)` loop is
/// unbounded; Vincenty's inverse is known not to converge for near-antipodal
/// endpoints. The iteration is capped at `MAX_LAMBDA_ITERATIONS` so a degenerate
/// input degrades to the last iterate instead of hanging the process. CesiumJS
/// guards the same case with a debug-only `Check` in `computeProperties` (mirrored
/// below), which release builds strip out.
const MAX_LAMBDA_ITERATIONS: usize = 1000;

#[allow(clippy::too_many_arguments)]
fn vincenty_inverse_formula(
    major: f64,
    minor: f64,
    first_longitude: f64,
    first_latitude: f64,
    second_longitude: f64,
    second_latitude: f64,
) -> InverseSolution {
    let eff = (major - minor) / major;
    let l = second_longitude - first_longitude;

    let u1 = ((1.0 - eff) * first_latitude.tan()).atan();
    let u2 = ((1.0 - eff) * second_latitude.tan()).atan();

    let cosine_u1 = u1.cos();
    let sine_u1 = u1.sin();
    let cosine_u2 = u2.cos();
    let sine_u2 = u2.sin();

    let cc = cosine_u1 * cosine_u2;
    let cs = cosine_u1 * sine_u2;
    let ss = sine_u1 * sine_u2;
    let sc = sine_u1 * cosine_u2;

    let mut lambda = l;

    // Values from the *last* iteration, i.e. the ones `lambda` was computed from.
    // CesiumJS declares these outside the loop for exactly this reason.
    let mut cosine_lambda = 0.0;
    let mut sine_lambda = 0.0;
    let mut sigma = 0.0;
    let mut cosine_sigma = 0.0;
    let mut sine_sigma = 0.0;
    let mut cosine_squared_alpha = 1.0;
    let mut cosine_twice_sigma_midpoint = 0.0;

    for _ in 0..MAX_LAMBDA_ITERATIONS {
        cosine_lambda = lambda.cos();
        sine_lambda = lambda.sin();

        let temp = cs - sc * cosine_lambda;
        sine_sigma =
            (cosine_u2 * cosine_u2 * sine_lambda * sine_lambda + temp * temp).sqrt();
        cosine_sigma = ss + cc * cosine_lambda;

        sigma = sine_sigma.atan2(cosine_sigma);

        let sine_alpha;

        if sine_sigma == 0.0 {
            sine_alpha = 0.0;
            cosine_squared_alpha = 1.0;
        } else {
            sine_alpha = (cc * sine_lambda) / sine_sigma;
            cosine_squared_alpha = 1.0 - sine_alpha * sine_alpha;
        }

        let lambda_dot = lambda;

        cosine_twice_sigma_midpoint = cosine_sigma - (2.0 * ss) / cosine_squared_alpha;

        if !cosine_twice_sigma_midpoint.is_finite() {
            cosine_twice_sigma_midpoint = 0.0;
        }

        lambda = l
            + compute_delta_lambda(
                eff,
                sine_alpha,
                cosine_squared_alpha,
                sigma,
                sine_sigma,
                cosine_sigma,
                cosine_twice_sigma_midpoint,
            );

        if (lambda - lambda_dot).abs() <= CesiumMath::EPSILON12 {
            break;
        }
    }

    let u_squared = (cosine_squared_alpha * (major * major - minor * minor)) / (minor * minor);
    let big_a = 1.0
        + (u_squared * (4096.0 + u_squared * (u_squared * (320.0 - 175.0 * u_squared) - 768.0)))
            / 16384.0;
    let big_b = (u_squared
        * (256.0 + u_squared * (u_squared * (74.0 - 47.0 * u_squared) - 128.0)))
        / 1024.0;

    let cosine_squared_twice_sigma_midpoint =
        cosine_twice_sigma_midpoint * cosine_twice_sigma_midpoint;
    let delta_sigma = big_b
        * sine_sigma
        * (cosine_twice_sigma_midpoint
            + (big_b
                * (cosine_sigma * (2.0 * cosine_squared_twice_sigma_midpoint - 1.0)
                    - (big_b
                        * cosine_twice_sigma_midpoint
                        * (4.0 * sine_sigma * sine_sigma - 3.0)
                        * (4.0 * cosine_squared_twice_sigma_midpoint - 3.0))
                        / 6.0))
                / 4.0);

    let distance = minor * big_a * (sigma - delta_sigma);

    let start_heading = (cosine_u2 * sine_lambda).atan2(cs - sc * cosine_lambda);
    let end_heading = (cosine_u1 * sine_lambda).atan2(cs * cosine_lambda - sc);

    InverseSolution {
        distance,
        start_heading,
        end_heading,
        u_squared,
    }
}

/// `setConstants` of CesiumJS.
///
/// `start_heading` is the value the inverse solution just stored on the geodesic;
/// CesiumJS reads it back off `this._startHeading`.
fn set_constants(geodesic: &mut EllipsoidGeodesic, start_heading: f64) {
    let u_squared = geodesic.u_squared;
    let a = geodesic.ellipsoid_ref.maximum_radius();
    let b = geodesic.ellipsoid_ref.minimum_radius();
    let f = (a - b) / a;

    let cosine_heading = start_heading.cos();
    let sine_heading = start_heading.sin();

    let tan_u = (1.0 - f) * geodesic.start.latitude.tan();

    let cosine_u = 1.0 / (1.0 + tan_u * tan_u).sqrt();
    let sine_u = cosine_u * tan_u;

    let sigma = tan_u.atan2(cosine_heading);

    let sine_alpha = cosine_u * sine_heading;
    let sine_squared_alpha = sine_alpha * sine_alpha;

    let cosine_squared_alpha = 1.0 - sine_squared_alpha;
    let cosine_alpha = cosine_squared_alpha.sqrt();

    let u2_over_4 = u_squared / 4.0;
    let u4_over_16 = u2_over_4 * u2_over_4;
    let u6_over_64 = u4_over_16 * u2_over_4;
    let u8_over_256 = u4_over_16 * u4_over_16;

    let a0 = 1.0 + u2_over_4 - (3.0 * u4_over_16) / 4.0 + (5.0 * u6_over_64) / 4.0
        - (175.0 * u8_over_256) / 64.0;
    let a1 = 1.0 - u2_over_4 + (15.0 * u4_over_16) / 8.0 - (35.0 * u6_over_64) / 8.0;
    let a2 = 1.0 - 3.0 * u2_over_4 + (35.0 * u4_over_16) / 4.0;
    let a3 = 1.0 - 5.0 * u2_over_4;

    let distance_ratio = a0 * sigma
        - (a1 * (2.0 * sigma).sin() * u2_over_4) / 2.0
        - (a2 * (4.0 * sigma).sin() * u4_over_16) / 16.0
        - (a3 * (6.0 * sigma).sin() * u6_over_64) / 48.0
        - ((8.0 * sigma).sin() * 5.0 * u8_over_256) / 512.0;

    geodesic.constants = GeodesicConstants {
        a,
        b,
        f,
        cosine_heading,
        sine_heading,
        tan_u,
        cosine_u,
        sine_u,
        sigma,
        sine_alpha,
        sine_squared_alpha,
        cosine_squared_alpha,
        cosine_alpha,
        u2_over_4,
        u4_over_16,
        u6_over_64,
        u8_over_256,
        a0,
        a1,
        a2,
        a3,
        distance_ratio,
    };
}

/// `computeProperties` of CesiumJS.
fn compute_properties(
    geodesic: &mut EllipsoidGeodesic,
    start: &Cartographic,
    end: &Cartographic,
) {
    let ellipsoid = &geodesic.ellipsoid_ref;

    //>>includeStart('debug', pragmas.debug)
    // `EllipsoidGeodesic` cannot represent a path whose endpoints are (nearly)
    // antipodal; CesiumJS rejects that here rather than inside Vincenty.
    if cfg!(debug_assertions) {
        let mut first = Cartesian3::default();
        let mut last = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(start, &mut first);
        ellipsoid.cartographic_to_cartesian(end, &mut last);
        first = Cartesian3::normalize_new(&first);
        last = Cartesian3::normalize_new(&last);
        let angle = Cartesian3::angle_between(&first, &last).abs();
        debug_assert!(
            (angle - std::f64::consts::PI).abs() >= 0.0125,
            "EllipsoidGeodesic endpoints must not be antipodal"
        );
    }
    //>>includeEnd('debug')

    let solution = vincenty_inverse_formula(
        ellipsoid.maximum_radius(),
        ellipsoid.minimum_radius(),
        start.longitude,
        start.latitude,
        end.longitude,
        end.latitude,
    );

    geodesic.distance = Some(solution.distance);
    geodesic.start_heading = Some(solution.start_heading);
    geodesic.end_heading = Some(solution.end_heading);
    geodesic.u_squared = solution.u_squared;

    geodesic.start = start.clone();
    geodesic.end = end.clone();
    geodesic.start.height = 0.0;
    geodesic.end.height = 0.0;

    set_constants(geodesic, solution.start_heading);
}

/// Computes the geodesic path between two points on an ellipsoid.
pub struct EllipsoidGeodesic {
    start: Cartographic,
    end: Cartographic,
    ellipsoid_ref: Ellipsoid,
    constants: GeodesicConstants,
    start_heading: Option<f64>,
    end_heading: Option<f64>,
    distance: Option<f64>,
    u_squared: f64,
}

impl EllipsoidGeodesic {
    /// Creates a new EllipsoidGeodesic from two cartographic positions.
    ///
    /// Mirrors `new EllipsoidGeodesic(start, end, ellipsoid)`: the two headings
    /// are *outputs* of the inverse solution, never inputs, so the properties are
    /// only computed when both endpoints are given.
    pub fn new(
        start: Option<Cartographic>,
        end: Option<Cartographic>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);

        let mut geodesic = Self {
            start: Cartographic::default(),
            end: Cartographic::default(),
            ellipsoid_ref: ellipsoid,
            constants: GeodesicConstants::default(),
            start_heading: None,
            end_heading: None,
            distance: None,
            u_squared: 0.0,
        };

        if let (Some(start), Some(end)) = (start, end) {
            compute_properties(&mut geodesic, &start, &end);
        }

        geodesic
    }

    /// Gets the start position.
    pub fn start(&self) -> &Cartographic {
        &self.start
    }

    /// Gets the end position.
    pub fn end(&self) -> &Cartographic {
        &self.end
    }

    /// Gets the surface distance.
    pub fn surface_distance(&self) -> f64 {
        // Check.defined("distance", this._distance)
        self.distance
            .expect("distance must be set (start and end points are required)")
    }

    /// Gets the start heading.
    pub fn start_heading(&self) -> f64 {
        // Check.defined("distance", this._distance)
        self.start_heading
            .expect("distance must be set (start and end points are required)")
    }

    /// Gets the end heading.
    pub fn end_heading(&self) -> f64 {
        // Check.defined("distance", this._distance)
        self.end_heading
            .expect("distance must be set (start and end points are required)")
    }

    /// Gets the ellipsoid.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid_ref
    }

    /// Sets the start and end points of the geodesic.
    pub fn set_end_points(&mut self, start: &Cartographic, end: &Cartographic) {
        compute_properties(self, start, end);
    }

    /// Provides the location of a point at the indicated portion along the geodesic.
    pub fn interpolate_using_fraction(&self, fraction: f64) -> Cartographic {
        self.interpolate_using_surface_distance(self.surface_distance() * fraction)
    }

    /// Provides the location of a point at the indicated distance along the geodesic.
    ///
    /// This is Vincenty's *direct* formula: the series in `s` inverts the
    /// distance/arc-length relation captured by `setConstants`, then the latitude
    /// follows from the reduced-length argument and the longitude from the
    /// remaining `deltaLambda` correction.
    pub fn interpolate_using_surface_distance(&self, distance: f64) -> Cartographic {
        let constants = &self.constants;

        let s = constants.distance_ratio + distance / constants.b;

        let cosine_2s = (2.0 * s).cos();
        let cosine_4s = (4.0 * s).cos();
        let cosine_6s = (6.0 * s).cos();
        let sine_2s = (2.0 * s).sin();
        let sine_4s = (4.0 * s).sin();
        let sine_6s = (6.0 * s).sin();
        let sine_8s = (8.0 * s).sin();

        let s2 = s * s;
        let s3 = s * s2;

        let u8_over_256 = constants.u8_over_256;
        let u2_over_4 = constants.u2_over_4;
        let u6_over_64 = constants.u6_over_64;
        let u4_over_16 = constants.u4_over_16;
        let mut sigma = (2.0 * s3 * u8_over_256 * cosine_2s) / 3.0
            + s * (1.0 - u2_over_4 + (7.0 * u4_over_16) / 4.0 - (15.0 * u6_over_64) / 4.0
                + (579.0 * u8_over_256) / 64.0
                - (u4_over_16 - (15.0 * u6_over_64) / 4.0 + (187.0 * u8_over_256) / 16.0)
                    * cosine_2s
                - ((5.0 * u6_over_64) / 4.0 - (115.0 * u8_over_256) / 16.0) * cosine_4s
                - (29.0 * u8_over_256 * cosine_6s) / 16.0)
            + (u2_over_4 / 2.0 - u4_over_16 + (71.0 * u6_over_64) / 32.0
                - (85.0 * u8_over_256) / 16.0)
                * sine_2s
            + ((5.0 * u4_over_16) / 16.0 - (5.0 * u6_over_64) / 4.0
                + (383.0 * u8_over_256) / 96.0)
                * sine_4s
            - s2 * ((u6_over_64 - (11.0 * u8_over_256) / 2.0) * sine_2s
                + (5.0 * u8_over_256 * sine_4s) / 2.0)
            + ((29.0 * u6_over_64) / 96.0 - (29.0 * u8_over_256) / 16.0) * sine_6s
            + (539.0 * u8_over_256 * sine_8s) / 1536.0;

        let theta = (sigma.sin() * constants.cosine_alpha).asin();
        let latitude = ((constants.a / constants.b) * theta.tan()).atan();

        // Redefine in terms of relative argument of latitude.
        sigma -= constants.sigma;

        let cosine_twice_sigma_midpoint = (2.0 * constants.sigma + sigma).cos();

        let sine_sigma = sigma.sin();
        let cosine_sigma = sigma.cos();

        let cc = constants.cosine_u * cosine_sigma;
        let ss = constants.sine_u * sine_sigma;

        let lambda =
            (sine_sigma * constants.sine_heading).atan2(cc - ss * constants.cosine_heading);

        let l = lambda
            - compute_delta_lambda(
                constants.f,
                constants.sine_alpha,
                constants.cosine_squared_alpha,
                sigma,
                sine_sigma,
                cosine_sigma,
                cosine_twice_sigma_midpoint,
            );

        Cartographic::new(self.start.longitude + l, latitude, 0.0)
    }
}
