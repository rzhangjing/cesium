//! Ellipsoid geodesic (great-circle path on the ellipsoid).
//!
//! Faithful port of CesiumJS `EllipsoidGeodesic.js`, which uses the Vincenty
//! inverse formula to compute the surface distance and headings between two
//! cartographic points, and a series expansion to interpolate intermediate
//! points at a given surface distance.

use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::math_utils::EPSILON12;

/// Precomputed constants for the geodesic series expansion.
#[derive(Debug, Clone, Default)]
struct GeodesicConstants {
    a: f64,
    b: f64,
    f: f64,
    cosine_heading: f64,
    sine_heading: f64,
    cosine_u: f64,
    sine_u: f64,
    sigma: f64,
    sine_alpha: f64,
    cosine_squared_alpha: f64,
    cosine_alpha: f64,
    u2_over4: f64,
    u4_over16: f64,
    u6_over64: f64,
    u8_over256: f64,
    distance_ratio: f64,
}

/// A geodesic on the ellipsoid connecting two planetodetic points.
///
/// Maps to CesiumJS `EllipsoidGeodesic`.
#[derive(Debug, Clone)]
pub struct EllipsoidGeodesic {
    start: Cartographic,
    end: Cartographic,
    start_heading: f64,
    end_heading: f64,
    distance: f64,
    constants: GeodesicConstants,
    maximum_radius: f64,
    minimum_radius: f64,
}

fn compute_c(f: f64, cosine_squared_alpha: f64) -> f64 {
    f * cosine_squared_alpha * (4.0 + f * (4.0 - 3.0 * cosine_squared_alpha)) / 16.0
}

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
                        * (2.0 * cosine_twice_sigma_midpoint * cosine_twice_sigma_midpoint
                            - 1.0)))
}

/// Result of the Vincenty inverse formula.
struct VincentyResult {
    distance: f64,
    start_heading: f64,
    end_heading: f64,
    u_squared: f64,
}

/// Intermediate values produced by the converging iteration.
struct VincentyIteration {
    sigma: f64,
    sine_sigma: f64,
    cosine_sigma: f64,
    cosine_squared_alpha: f64,
    cosine_twice_sigma_midpoint: f64,
    lambda: f64,
}

fn vincenty_inverse_formula(
    major: f64,
    minor: f64,
    first_longitude: f64,
    first_latitude: f64,
    second_longitude: f64,
    second_latitude: f64,
) -> VincentyResult {
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

    let iter = loop {
        let cosine_lambda = lambda.cos();
        let sine_lambda = lambda.sin();

        let temp = cs - sc * cosine_lambda;
        let sine_sigma =
            (cosine_u2 * cosine_u2 * sine_lambda * sine_lambda + temp * temp).sqrt();
        let cosine_sigma = ss + cc * cosine_lambda;

        let sigma = sine_sigma.atan2(cosine_sigma);

        let (sine_alpha, cosine_squared_alpha) = if sine_sigma == 0.0 {
            (0.0, 1.0)
        } else {
            let sa = cc * sine_lambda / sine_sigma;
            (sa, 1.0 - sa * sa)
        };

        let lambda_dot = lambda;

        let mut cosine_twice_sigma_midpoint =
            cosine_sigma - 2.0 * ss / cosine_squared_alpha;
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

        if (lambda - lambda_dot).abs() <= EPSILON12 {
            break VincentyIteration {
                sigma,
                sine_sigma,
                cosine_sigma,
                cosine_squared_alpha,
                cosine_twice_sigma_midpoint,
                lambda,
            };
        }
    };

    let cosine_squared_alpha = iter.cosine_squared_alpha;
    let u_squared =
        cosine_squared_alpha * (major * major - minor * minor) / (minor * minor);
    let big_a = 1.0
        + u_squared
            * (4096.0 + u_squared * (u_squared * (320.0 - 175.0 * u_squared) - 768.0))
            / 16384.0;
    let big_b = u_squared
        * (256.0 + u_squared * (u_squared * (74.0 - 47.0 * u_squared) - 128.0))
        / 1024.0;

    let cosine_twice_sigma_midpoint = iter.cosine_twice_sigma_midpoint;
    let cosine_squared_twice_sigma_midpoint =
        cosine_twice_sigma_midpoint * cosine_twice_sigma_midpoint;
    let sine_sigma = iter.sine_sigma;
    let cosine_sigma = iter.cosine_sigma;
    let delta_sigma = big_b
        * sine_sigma
        * (cosine_twice_sigma_midpoint
            + big_b
                * (cosine_sigma * (2.0 * cosine_squared_twice_sigma_midpoint - 1.0)
                    - big_b
                        * cosine_twice_sigma_midpoint
                        * (4.0 * sine_sigma * sine_sigma - 3.0)
                        * (4.0 * cosine_squared_twice_sigma_midpoint - 3.0)
                        / 6.0)
                / 4.0);

    let distance = minor * big_a * (iter.sigma - delta_sigma);

    let cosine_lambda = iter.lambda.cos();
    let sine_lambda = iter.lambda.sin();
    let start_heading = (cosine_u2 * sine_lambda).atan2(cs - sc * cosine_lambda);
    let end_heading = (cosine_u1 * sine_lambda).atan2(cs * cosine_lambda - sc);

    VincentyResult {
        distance,
        start_heading,
        end_heading,
        u_squared,
    }
}

fn set_constants(
    start: &Cartographic,
    start_heading: f64,
    u_squared: f64,
    maximum_radius: f64,
    minimum_radius: f64,
) -> GeodesicConstants {
    let a = maximum_radius;
    let b = minimum_radius;
    let f = (a - b) / a;

    let cosine_heading = start_heading.cos();
    let sine_heading = start_heading.sin();

    let tan_u = (1.0 - f) * start.latitude.tan();

    let cosine_u = 1.0 / (1.0 + tan_u * tan_u).sqrt();
    let sine_u = cosine_u * tan_u;

    let sigma = tan_u.atan2(cosine_heading);

    let sine_alpha = cosine_u * sine_heading;
    let sine_squared_alpha = sine_alpha * sine_alpha;

    let cosine_squared_alpha = 1.0 - sine_squared_alpha;
    let cosine_alpha = cosine_squared_alpha.sqrt();

    let u2_over4 = u_squared / 4.0;
    let u4_over16 = u2_over4 * u2_over4;
    let u6_over64 = u4_over16 * u2_over4;
    let u8_over256 = u4_over16 * u4_over16;

    let a0 = 1.0 + u2_over4 - 3.0 * u4_over16 / 4.0 + 5.0 * u6_over64 / 4.0
        - 175.0 * u8_over256 / 64.0;
    let a1 = 1.0 - u2_over4 + 15.0 * u4_over16 / 8.0 - 35.0 * u6_over64 / 8.0;
    let a2 = 1.0 - 3.0 * u2_over4 + 35.0 * u4_over16 / 4.0;
    let a3 = 1.0 - 5.0 * u2_over4;

    let distance_ratio = a0 * sigma
        - a1 * (2.0 * sigma).sin() * u2_over4 / 2.0
        - a2 * (4.0 * sigma).sin() * u4_over16 / 16.0
        - a3 * (6.0 * sigma).sin() * u6_over64 / 48.0
        - (8.0 * sigma).sin() * 5.0 * u8_over256 / 512.0;

    GeodesicConstants {
        a,
        b,
        f,
        cosine_heading,
        sine_heading,
        cosine_u,
        sine_u,
        sigma,
        sine_alpha,
        cosine_squared_alpha,
        cosine_alpha,
        u2_over4,
        u4_over16,
        u6_over64,
        u8_over256,
        distance_ratio,
    }
}

impl EllipsoidGeodesic {
    /// Creates a geodesic connecting `start` to `end` on the given ellipsoid.
    ///
    /// Maps to the `EllipsoidGeodesic` constructor / `setEndPoints`.
    pub fn new(start: Cartographic, end: Cartographic, ellipsoid: &Ellipsoid) -> Self {
        Self::from_radii(start, end, ellipsoid.maximum_radius(), ellipsoid.minimum_radius())
    }

    /// Creates a geodesic from explicit ellipsoid radii.
    fn from_radii(
        start: Cartographic,
        end: Cartographic,
        maximum_radius: f64,
        minimum_radius: f64,
    ) -> Self {
        let vincenty = vincenty_inverse_formula(
            maximum_radius,
            minimum_radius,
            start.longitude,
            start.latitude,
            end.longitude,
            end.latitude,
        );

        let mut start0 = start;
        let mut end0 = end;
        start0.height = 0.0;
        end0.height = 0.0;

        let constants = set_constants(
            &start0,
            vincenty.start_heading,
            vincenty.u_squared,
            maximum_radius,
            minimum_radius,
        );

        Self {
            start: start0,
            end: end0,
            start_heading: vincenty.start_heading,
            end_heading: vincenty.end_heading,
            distance: vincenty.distance,
            constants,
            maximum_radius,
            minimum_radius,
        }
    }

    /// Resets the endpoints of the geodesic.
    pub fn set_end_points(&mut self, start: Cartographic, end: Cartographic) {
        *self = Self::from_radii(start, end, self.maximum_radius, self.minimum_radius);
    }

    /// The surface distance between the start and end points.
    pub fn surface_distance(&self) -> f64 {
        self.distance
    }

    /// The heading at the start point.
    pub fn start_heading(&self) -> f64 {
        self.start_heading
    }

    /// The heading at the end point.
    pub fn end_heading(&self) -> f64 {
        self.end_heading
    }

    /// The start point of the geodesic.
    pub fn start(&self) -> Cartographic {
        self.start
    }

    /// The end point of the geodesic.
    pub fn end(&self) -> Cartographic {
        self.end
    }

    /// Interpolates a point at the given fraction (0..1) along the geodesic.
    pub fn interpolate_using_fraction(&self, fraction: f64) -> Cartographic {
        self.interpolate_using_surface_distance(self.distance * fraction)
    }

    /// Interpolates a point at the given surface distance from the start.
    ///
    /// Maps to `EllipsoidGeodesic.interpolateUsingSurfaceDistance`.
    pub fn interpolate_using_surface_distance(&self, distance: f64) -> Cartographic {
        let c = &self.constants;

        let s = c.distance_ratio + distance / c.b;

        let cosine2s = (2.0 * s).cos();
        let cosine4s = (4.0 * s).cos();
        let cosine6s = (6.0 * s).cos();
        let sine2s = (2.0 * s).sin();
        let sine4s = (4.0 * s).sin();
        let sine6s = (6.0 * s).sin();
        let sine8s = (8.0 * s).sin();

        let s2 = s * s;
        let s3 = s * s2;

        let u8_over256 = c.u8_over256;
        let u2_over4 = c.u2_over4;
        let u6_over64 = c.u6_over64;
        let u4_over16 = c.u4_over16;

        let mut sigma = (2.0 * s3 * u8_over256 * cosine2s) / 3.0
            + s * (1.0 - u2_over4 + 7.0 * u4_over16 / 4.0 - 15.0 * u6_over64 / 4.0
                + 579.0 * u8_over256 / 64.0
                - (u4_over16 - 15.0 * u6_over64 / 4.0 + 187.0 * u8_over256 / 16.0)
                    * cosine2s
                - (5.0 * u6_over64 / 4.0 - 115.0 * u8_over256 / 16.0) * cosine4s
                - 29.0 * u8_over256 * cosine6s / 16.0)
            + (u2_over4 / 2.0 - u4_over16 + 71.0 * u6_over64 / 32.0
                - 85.0 * u8_over256 / 16.0)
                * sine2s
            + (5.0 * u4_over16 / 16.0 - 5.0 * u6_over64 / 4.0
                + 383.0 * u8_over256 / 96.0)
                * sine4s
            - s2 * ((u6_over64 - 11.0 * u8_over256 / 2.0) * sine2s
                + 5.0 * u8_over256 * sine4s / 2.0)
            + (29.0 * u6_over64 / 96.0 - 29.0 * u8_over256 / 16.0) * sine6s
            + 539.0 * u8_over256 * sine8s / 1536.0;

        let theta = (sigma.sin() * c.cosine_alpha).asin();
        let latitude = ((c.a / c.b) * theta.tan()).atan();

        // Redefine in terms of relative argument of latitude.
        sigma -= c.sigma;

        let cosine_twice_sigma_midpoint = (2.0 * c.sigma + sigma).cos();

        let sine_sigma = sigma.sin();
        let cosine_sigma = sigma.cos();

        let cc = c.cosine_u * cosine_sigma;
        let ss = c.sine_u * sine_sigma;

        let lambda =
            (sine_sigma * c.sine_heading).atan2(cc - ss * c.cosine_heading);

        let l = lambda
            - compute_delta_lambda(
                c.f,
                c.sine_alpha,
                c.cosine_squared_alpha,
                sigma,
                sine_sigma,
                cosine_sigma,
                cosine_twice_sigma_midpoint,
            );

        Cartographic::from_radians(self.start.longitude + l, latitude, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math_utils::to_radians;

    #[test]
    fn test_surface_distance_equator() {
        // 1 degree along the equator on WGS84 ~ 111319.49 m
        let ell = Ellipsoid::WGS84;
        let start = Cartographic::from_degrees(0.0, 0.0, 0.0);
        let end = Cartographic::from_degrees(1.0, 0.0, 0.0);
        let g = EllipsoidGeodesic::new(start, end, &ell);
        let expected = to_radians(1.0) * ell.maximum_radius();
        assert!(
            (g.surface_distance() - expected).abs() < 1.0,
            "distance {}",
            g.surface_distance()
        );
    }

    #[test]
    fn test_interpolate_midpoint() {
        let ell = Ellipsoid::WGS84;
        let start = Cartographic::from_degrees(0.0, 0.0, 0.0);
        let end = Cartographic::from_degrees(10.0, 0.0, 0.0);
        let g = EllipsoidGeodesic::new(start, end, &ell);
        let mid = g.interpolate_using_fraction(0.5);
        assert!(
            (to_radians(5.0) - mid.longitude).abs() < 1e-6,
            "mid lon {}",
            mid.longitude
        );
        assert!(mid.latitude.abs() < 1e-6, "mid lat {}", mid.latitude);
    }

    #[test]
    fn test_interpolate_endpoints() {
        let ell = Ellipsoid::WGS84;
        let start = Cartographic::from_degrees(-105.0, 40.0, 0.0);
        let end = Cartographic::from_degrees(-100.0, 38.0, 0.0);
        let g = EllipsoidGeodesic::new(start, end, &ell);
        let p0 = g.interpolate_using_surface_distance(0.0);
        assert!((p0.longitude - start.longitude).abs() < 1e-9);
        assert!((p0.latitude - start.latitude).abs() < 1e-9);
        let p1 = g.interpolate_using_surface_distance(g.surface_distance());
        assert!((p1.longitude - end.longitude).abs() < 1e-6);
        assert!((p1.latitude - end.latitude).abs() < 1e-6);
    }
}
