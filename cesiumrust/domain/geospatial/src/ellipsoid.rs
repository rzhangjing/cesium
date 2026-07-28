//! Ellipsoid - a quadratic surface defined in Cartesian coordinates.
//! Maps to CesiumJS `Core/Ellipsoid.js` + `Core/scaleToGeodeticSurface.js`

use crate::cartographic::Cartographic;
use crate::math_utils::{self, EPSILON1, EPSILON12, EPSILON14, EPSILON15, LUNAR_RADIUS, TWO_PI};
use crate::rectangle::Rectangle;
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// A quadratic surface defined in Cartesian coordinates by the equation
/// `(x / a)^2 + (y / b)^2 + (z / c)^2 = 1`.
/// Primarily used to represent the shape of planetary bodies.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ellipsoid {
    /// The radii of the ellipsoid (x, y, z).
    radii: DVec3,
    /// The squared radii.
    radii_squared: DVec3,
    /// The radii raised to the fourth power.
    radii_to_the_fourth: DVec3,
    /// One over the radii.
    one_over_radii: DVec3,
    /// One over the squared radii.
    one_over_radii_squared: DVec3,
    /// The minimum radius.
    minimum_radius: f64,
    /// The maximum radius.
    maximum_radius: f64,
    /// Tolerance for closeness to the center.
    center_tolerance_squared: f64,
    /// squaredXOverSquaredZ
    squared_x_over_squared_z: f64,
}

/// Normalizes a Cartesian3 by dividing each component by its magnitude.
///
/// This is a bit-exact port of CesiumJS `Cartesian3.normalize`, which computes
/// `component / magnitude` (a single correctly-rounded IEEE-754 division per
/// component). glam's `DVec3::normalize` instead computes
/// `component * (1.0 / length)` (multiply-by-reciprocal, two roundings), which
/// can differ from the CesiumJS result by 1 ulp. For verification against the
/// original CesiumJS Specs (the ground truth), the direct-division form is
/// required. Use this helper wherever CesiumJS `Cartesian3.normalize` is ported.
#[inline]
pub fn normalize_cartesian3(v: DVec3) -> DVec3 {
    let magnitude = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    DVec3::new(v.x / magnitude, v.y / magnitude, v.z / magnitude)
}

impl Ellipsoid {
    /// WGS84 ellipsoid: a = 6378137.0, b = 6378137.0, c = 6356752.3142451793
    #[allow(clippy::excessive_precision)]
    pub const WGS84: Self = Self::from_radii_unchecked(
        6378137.0,
        6378137.0,
        6356752.3142451793,
    );

    /// Unit sphere (radius 1 in all directions).
    pub const UNIT_SPHERE: Self = Self::from_radii_unchecked(1.0, 1.0, 1.0);

    /// Moon ellipsoid. Matches CesiumJS `Ellipsoid.MOON`: a sphere of radius
    /// `CesiumMath.LUNAR_RADIUS` (1737400.0 m). (Note: this is NOT the IAU 2000
    /// triaxial Moon ellipsoid; CesiumJS models the Moon as a sphere.)
    pub const MOON: Self = Self::from_radii_unchecked(LUNAR_RADIUS, LUNAR_RADIUS, LUNAR_RADIUS);

    /// Degenerate ellipsoid with all radii zero.
    /// Maps to CesiumJS `Ellipsoid.ZERO`.
    pub const ZERO: Self = Self::from_radii_unchecked(0.0, 0.0, 0.0);

    /// Creates an Ellipsoid from radii. Const version for static initialization.
    pub(crate) const fn from_radii_unchecked(x: f64, y: f64, z: f64) -> Self {
        let radii_squared = DVec3::new(x * x, y * y, z * z);
        let radii_to_the_fourth = DVec3::new(x * x * x * x, y * y * y * y, z * z * z * z);
        let one_over_radii = DVec3::new(
            if x == 0.0 { 0.0 } else { 1.0 / x },
            if y == 0.0 { 0.0 } else { 1.0 / y },
            if z == 0.0 { 0.0 } else { 1.0 / z },
        );
        let one_over_radii_squared = DVec3::new(
            if x == 0.0 { 0.0 } else { 1.0 / (x * x) },
            if y == 0.0 { 0.0 } else { 1.0 / (y * y) },
            if z == 0.0 { 0.0 } else { 1.0 / (z * z) },
        );
        let minimum_radius = if x < y {
            if x < z { x } else { z }
        } else if y < z {
            y
        } else {
            z
        };
        let maximum_radius = if x > y {
            if x > z { x } else { z }
        } else if y > z {
            y
        } else {
            z
        };
        let squared_x_over_squared_z = if radii_squared.z != 0.0 {
            radii_squared.x / radii_squared.z
        } else {
            0.0
        };

        Self {
            radii: DVec3::new(x, y, z),
            radii_squared,
            radii_to_the_fourth,
            one_over_radii,
            one_over_radii_squared,
            minimum_radius,
            maximum_radius,
            center_tolerance_squared: EPSILON1,
            squared_x_over_squared_z,
        }
    }

    /// Creates a new Ellipsoid from radii values.
    /// Maps to `new Ellipsoid(x, y, z)`
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        assert!(x >= 0.0, "x radius must be >= 0");
        assert!(y >= 0.0, "y radius must be >= 0");
        assert!(z >= 0.0, "z radius must be >= 0");
        Self::from_radii_unchecked(x, y, z)
    }

    /// Creates an Ellipsoid from a DVec3 of radii.
    /// Maps to `Ellipsoid.fromCartesian3`
    pub fn from_cartesian3(radii: DVec3) -> Self {
        Self::new(radii.x, radii.y, radii.z)
    }

    // --- Getters ---

    #[inline]
    pub fn radii(&self) -> DVec3 {
        self.radii
    }

    #[inline]
    pub fn radii_squared(&self) -> DVec3 {
        self.radii_squared
    }

    #[inline]
    pub fn radii_to_the_fourth(&self) -> DVec3 {
        self.radii_to_the_fourth
    }

    #[inline]
    pub fn one_over_radii(&self) -> DVec3 {
        self.one_over_radii
    }

    #[inline]
    pub fn one_over_radii_squared(&self) -> DVec3 {
        self.one_over_radii_squared
    }

    #[inline]
    pub fn minimum_radius(&self) -> f64 {
        self.minimum_radius
    }

    #[inline]
    pub fn maximum_radius(&self) -> f64 {
        self.maximum_radius
    }

    #[inline]
    pub fn squared_x_over_squared_z(&self) -> f64 {
        self.squared_x_over_squared_z
    }

    // --- Core algorithms ---

    /// Computes the normal of the plane tangent to the surface of the ellipsoid
    /// at the provided cartographic position.
    /// Maps to `Ellipsoid.geodeticSurfaceNormalCartographic`
    pub fn geodetic_surface_normal_cartographic(&self, cartographic: &Cartographic) -> DVec3 {
        let longitude = cartographic.longitude;
        let latitude = cartographic.latitude;
        let cos_latitude = latitude.cos();

        let x = cos_latitude * longitude.cos();
        let y = cos_latitude * longitude.sin();
        let z = latitude.sin();

        normalize_cartesian3(DVec3::new(x, y, z))
    }

    /// Computes the normal of the plane tangent to the surface of the ellipsoid
    /// at the provided Cartesian position.
    /// Maps to `Ellipsoid.geodeticSurfaceNormal`
    /// Returns None if the position is at the center of the ellipsoid.
    pub fn geodetic_surface_normal(&self, cartesian: DVec3) -> Option<DVec3> {
        if cartesian.abs_diff_eq(DVec3::ZERO, EPSILON14) {
            return None;
        }
        let result = cartesian * self.one_over_radii_squared;
        Some(normalize_cartesian3(result))
    }

    /// Converts the provided cartographic to Cartesian representation.
    /// Maps to `Ellipsoid.cartographicToCartesian`
    pub fn cartographic_to_cartesian(&self, cartographic: &Cartographic) -> DVec3 {
        let n = self.geodetic_surface_normal_cartographic(cartographic);
        let k = self.radii_squared * n;
        let gamma = (n.dot(k)).sqrt();
        let k_scaled = k / gamma;
        let n_scaled = n * cartographic.height;
        k_scaled + n_scaled
    }

    /// Converts an array of cartographics to Cartesian positions.
    /// Maps to `Ellipsoid.cartographicArrayToCartesianArray`
    pub fn cartographic_array_to_cartesian_array(
        &self,
        cartographics: &[Cartographic],
    ) -> Vec<DVec3> {
        cartographics
            .iter()
            .map(|c| self.cartographic_to_cartesian(c))
            .collect()
    }

    /// Converts the provided Cartesian to cartographic representation.
    /// Maps to `Ellipsoid.cartesianToCartographic`
    /// Returns None if the position is at the center of the ellipsoid.
    pub fn cartesian_to_cartographic(&self, cartesian: DVec3) -> Option<Cartographic> {
        let p = self.scale_to_geodetic_surface(cartesian)?;
        let n = self.geodetic_surface_normal(p)?;
        let h = cartesian - p;

        let longitude = n.y.atan2(n.x);
        let latitude = n.z.asin();
        let height = math_utils::sign(h.dot(cartesian)) * h.length();

        Some(Cartographic {
            longitude,
            latitude,
            height,
        })
    }

    /// Converts an array of Cartesians to cartographic positions.
    /// Maps to `Ellipsoid.cartesianArrayToCartographicArray`
    pub fn cartesian_array_to_cartographic_array(
        &self,
        cartesians: &[DVec3],
    ) -> Vec<Option<Cartographic>> {
        cartesians
            .iter()
            .map(|c| self.cartesian_to_cartographic(*c))
            .collect()
    }

    /// Scales the provided Cartesian position along the geodetic surface normal
    /// so that it is on the surface of this ellipsoid.
    /// Maps to `Ellipsoid.scaleToGeodeticSurface` → `scaleToGeodeticSurface.js`
    /// Returns None if the position is at the center of the ellipsoid.
    pub fn scale_to_geodetic_surface(&self, cartesian: DVec3) -> Option<DVec3> {
        scale_to_geodetic_surface(
            cartesian,
            self.one_over_radii,
            self.one_over_radii_squared,
            self.center_tolerance_squared,
        )
    }

    /// Scales the provided Cartesian position along the geodetic surface normal
    /// so that it is on the surface of this ellipsoid. If the position is at the
    /// center, returns the center.
    /// Maps to `Ellipsoid.scaleToGeocentricSurface`
    pub fn scale_to_geocentric_surface(&self, cartesian: DVec3) -> Option<DVec3> {
        let position_x = cartesian.x;
        let position_y = cartesian.y;
        let position_z = cartesian.z;

        let beta = 1.0
            / ((position_x * position_x) * self.one_over_radii_squared.x
                + (position_y * position_y) * self.one_over_radii_squared.y
                + (position_z * position_z) * self.one_over_radii_squared.z)
                .sqrt();

        if !beta.is_finite() {
            return None;
        }

        Some(cartesian * beta)
    }

    /// Computes the intersection of a ray with the ellipsoid.
    /// Returns (start, stop) interval of parametric distances along the ray, or None.
    /// Faithful port of `IntersectionTests.rayEllipsoid`.
    pub fn intersection(&self, ray_origin: DVec3, ray_direction: DVec3) -> Option<(f64, f64)> {
        let q = ray_origin * self.one_over_radii;
        let w = ray_direction * self.one_over_radii;

        let q2 = q.length_squared();
        let qw = q.dot(w);

        if q2 > 1.0 {
            // Outside ellipsoid.
            if qw >= 0.0 {
                // Looking outward or tangent (0 intersections).
                return None;
            }

            // qw < 0.0
            let qw2 = qw * qw;
            let difference = q2 - 1.0; // Positively valued.
            let w2 = w.length_squared();
            let product = w2 * difference;

            if qw2 < product {
                // Imaginary roots (0 intersections).
                return None;
            } else if qw2 > product {
                // Distinct roots (2 intersections).
                let discriminant = qw * qw - product;
                let temp = -qw + discriminant.sqrt(); // Avoid cancellation.
                let root0 = temp / w2;
                let root1 = difference / temp;
                if root0 < root1 {
                    Some((root0, root1))
                } else {
                    Some((root1, root0))
                }
            } else {
                // qw2 == product. Repeated roots (2 intersections).
                let root = (difference / w2).sqrt();
                Some((root, root))
            }
        } else if q2 < 1.0 {
            // Inside ellipsoid (2 intersections).
            let difference = q2 - 1.0; // Negatively valued.
            let w2 = w.length_squared();
            let product = w2 * difference; // Negatively valued.

            let discriminant = qw * qw - product;
            let temp = -qw + discriminant.sqrt(); // Positively valued.
            Some((0.0, temp / w2))
        } else {
            // q2 == 1.0. On ellipsoid.
            if qw < 0.0 {
                // Looking inward.
                let w2 = w.length_squared();
                Some((0.0, -qw / w2))
            } else {
                // qw >= 0.0. Looking outward or tangent.
                None
            }
        }
    }

    /// Transforms a Cartesian X, Y, Z position to the ellipsoid-scaled space by
    /// multiplying its components by `oneOverRadii`.
    /// Maps to `Ellipsoid.transformPositionToScaledSpace`
    pub fn transform_position_to_scaled_space(&self, position: DVec3) -> DVec3 {
        position * self.one_over_radii
    }

    /// Transforms a Cartesian X, Y, Z position from the ellipsoid-scaled space by
    /// multiplying its components by `radii`.
    /// Maps to `Ellipsoid.transformPositionFromScaledSpace`
    pub fn transform_position_from_scaled_space(&self, position: DVec3) -> DVec3 {
        position * self.radii
    }

    /// Computes the unit vector directed from the center of this ellipsoid toward
    /// the provided Cartesian position (i.e. the geocentric surface normal).
    /// Maps to `Ellipsoid.geocentricSurfaceNormal` (= `Cartesian3.normalize`)
    pub fn geocentric_surface_normal(&self, cartesian: DVec3) -> DVec3 {
        normalize_cartesian3(cartesian)
    }

    /// Computes a point which is the intersection of the surface normal with the z-axis.
    /// Maps to `Ellipsoid.getSurfaceNormalIntersectionWithZAxis`
    ///
    /// Returns `None` if the intersection point lies outside the ellipsoid
    /// (shrunk by `buffer`).
    ///
    /// # Panics
    /// Panics if the ellipsoid is not an ellipsoid of revolution (radii.x != radii.y)
    /// or if radii.z is not greater than 0.
    pub fn get_surface_normal_intersection_with_z_axis(
        &self,
        position: DVec3,
        buffer: Option<f64>,
    ) -> Option<DVec3> {
        assert!(
            math_utils::equals_epsilon(self.radii.x, self.radii.y, EPSILON15, 0.0),
            "Ellipsoid must be an ellipsoid of revolution (radii.x == radii.y)"
        );
        assert!(self.radii.z > 0.0, "Ellipsoid.radii.z must be greater than 0");

        let buffer = buffer.unwrap_or(0.0);
        let squared_x_over_squared_z = self.squared_x_over_squared_z;

        let z = position.z * (1.0 - squared_x_over_squared_z);

        if z.abs() >= self.radii.z - buffer {
            return None;
        }

        Some(DVec3::new(0.0, 0.0, z))
    }

    /// Computes the ellipsoid curvatures at a given position on the surface.
    /// Maps to `Ellipsoid.getLocalCurvature`
    /// Returns the local curvature (east, north) as a `DVec2`, or `None` if the
    /// surface-normal/z-axis intersection is outside the ellipsoid.
    pub fn get_local_curvature(&self, surface_position: DVec3) -> Option<DVec2> {
        let prime_vertical_endpoint = self
            .get_surface_normal_intersection_with_z_axis(surface_position, Some(0.0))?;
        let prime_vertical_radius = surface_position.distance(prime_vertical_endpoint);
        // meridional radius = (1 - e^2) * primeVerticalRadius^3 / a^2
        // where 1 - e^2 = b^2 / a^2, so meridional = b^2 * primeVerticalRadius^3 / a^4
        //   = (b * primeVerticalRadius / a^2)^2 * primeVertical
        let radius_ratio =
            (self.minimum_radius * prime_vertical_radius) / self.maximum_radius.powi(2);
        let meridional_radius = prime_vertical_radius * radius_ratio.powi(2);

        Some(DVec2::new(
            1.0 / prime_vertical_radius,
            1.0 / meridional_radius,
        ))
    }

    /// Computes an approximation of the surface area of a rectangle on the surface
    /// of this ellipsoid using Gauss-Legendre 10th order quadrature.
    /// Maps to `Ellipsoid.surfaceArea`
    pub fn surface_area(&self, rectangle: &Rectangle) -> f64 {
        let min_longitude = rectangle.west;
        let mut max_longitude = rectangle.east;
        let min_latitude = rectangle.south;
        let max_latitude = rectangle.north;

        while max_longitude < min_longitude {
            max_longitude += TWO_PI;
        }

        let a2 = self.radii_squared.x;
        let b2 = self.radii_squared.y;
        let c2 = self.radii_squared.z;
        let a2b2 = a2 * b2;

        gauss_legendre_quadrature(min_latitude, max_latitude, |lat| {
            // phi represents the angle measured from the north pole
            // sin(phi) = sin(pi / 2 - lat) = cos(lat), cos(phi) is similar
            let sin_phi = lat.cos();
            let cos_phi = lat.sin();
            lat.cos()
                * gauss_legendre_quadrature(min_longitude, max_longitude, |lon| {
                    let cos_theta = lon.cos();
                    let sin_theta = lon.sin();
                    (a2b2 * cos_phi * cos_phi
                        + c2
                            * (b2 * cos_theta * cos_theta + a2 * sin_theta * sin_theta)
                            * sin_phi
                            * sin_phi)
                        .sqrt()
                })
        })
    }

    /// The number of elements used to pack the object into an array.
    /// Maps to `Ellipsoid.packedLength`
    pub const PACKED_LENGTH: usize = 3;

    /// Stores the provided instance into the provided array.
    /// Maps to `Ellipsoid.pack`
    pub fn pack(&self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = self.radii.x;
        array[starting_index + 1] = self.radii.y;
        array[starting_index + 2] = self.radii.z;
    }

    /// Retrieves an instance from a packed array.
    /// Maps to `Ellipsoid.unpack`
    pub fn unpack(array: &[f64], starting_index: usize) -> Self {
        Self::new(
            array[starting_index],
            array[starting_index + 1],
            array[starting_index + 2],
        )
    }
}

impl std::fmt::Display for Ellipsoid {
    /// Formats as `(radii.x, radii.y, radii.z)`.
    /// Maps to `Ellipsoid.toString`
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.radii.x, self.radii.y, self.radii.z)
    }
}

/// Scales the provided Cartesian position along the geodetic surface normal
/// so that it is on the surface of the ellipsoid.
/// Direct port of CesiumJS `scaleToGeodeticSurface.js` using Newton's method.
fn scale_to_geodetic_surface(
    cartesian: DVec3,
    one_over_radii: DVec3,
    one_over_radii_squared: DVec3,
    center_tolerance_squared: f64,
) -> Option<DVec3> {
    let position_x = cartesian.x;
    let position_y = cartesian.y;
    let position_z = cartesian.z;

    let one_over_radii_x = one_over_radii.x;
    let one_over_radii_y = one_over_radii.y;
    let one_over_radii_z = one_over_radii.z;

    let x2 = position_x * position_x * one_over_radii_x * one_over_radii_x;
    let y2 = position_y * position_y * one_over_radii_y * one_over_radii_y;
    let z2 = position_z * position_z * one_over_radii_z * one_over_radii_z;

    // Compute the squared ellipsoid norm.
    let squared_norm = x2 + y2 + z2;
    let ratio = (1.0 / squared_norm).sqrt();

    // As an initial approximation, assume that the radial intersection is the projection point.
    let intersection = cartesian * ratio;

    // If the position is near the center, the iteration will not converge.
    if squared_norm < center_tolerance_squared {
        if !ratio.is_finite() {
            return None;
        }
        return Some(intersection);
    }

    let one_over_radii_squared_x = one_over_radii_squared.x;
    let one_over_radii_squared_y = one_over_radii_squared.y;
    let one_over_radii_squared_z = one_over_radii_squared.z;

    // Use the gradient at the intersection point in place of the true unit normal.
    let gradient = DVec3::new(
        intersection.x * one_over_radii_squared_x * 2.0,
        intersection.y * one_over_radii_squared_y * 2.0,
        intersection.z * one_over_radii_squared_z * 2.0,
    );

    // Compute the initial guess at the normal vector multiplier, lambda.
    let mut lambda =
        ((1.0 - ratio) * cartesian.length()) / (0.5 * gradient.length());
    let mut correction: f64 = 0.0;

    let mut x_multiplier: f64;
    let mut y_multiplier: f64;
    let mut z_multiplier: f64;

    loop {
        lambda -= correction;

        x_multiplier = 1.0 / (1.0 + lambda * one_over_radii_squared_x);
        y_multiplier = 1.0 / (1.0 + lambda * one_over_radii_squared_y);
        z_multiplier = 1.0 / (1.0 + lambda * one_over_radii_squared_z);

        let x_multiplier2 = x_multiplier * x_multiplier;
        let y_multiplier2 = y_multiplier * y_multiplier;
        let z_multiplier2 = z_multiplier * z_multiplier;

        let x_multiplier3 = x_multiplier2 * x_multiplier;
        let y_multiplier3 = y_multiplier2 * y_multiplier;
        let z_multiplier3 = z_multiplier2 * z_multiplier;

        let func =
            x2 * x_multiplier2 + y2 * y_multiplier2 + z2 * z_multiplier2 - 1.0;

        // "denominator" for velocity and acceleration computations
        let denominator = x2 * x_multiplier3 * one_over_radii_squared_x
            + y2 * y_multiplier3 * one_over_radii_squared_y
            + z2 * z_multiplier3 * one_over_radii_squared_z;

        let derivative = -2.0 * denominator;
        correction = func / derivative;

        if func.abs() <= EPSILON12 {
            break;
        }
    }

    Some(DVec3::new(
        position_x * x_multiplier,
        position_y * y_multiplier,
        position_z * z_multiplier,
    ))
}

/// Gauss-Legendre 10th order quadrature abscissas (last element unused, present
/// to mirror the CesiumJS table layout).
const GAUSS_LEGENDRE_ABSCISSAS: [f64; 6] = [
    0.14887433898163,
    0.43339539412925,
    0.67940956829902,
    0.86506336668898,
    0.97390652851717,
    0.0,
];

/// Gauss-Legendre 10th order quadrature weights.
const GAUSS_LEGENDRE_WEIGHTS: [f64; 6] = [
    0.29552422471475,
    0.26926671930999,
    0.21908636251598,
    0.14945134915058,
    0.066671344308684,
    0.0,
];

/// Compute the 10th order Gauss-Legendre Quadrature of the given definite integral.
/// Maps to CesiumJS `gaussLegendreQuadrature` (private helper in Ellipsoid.js).
fn gauss_legendre_quadrature<F: Fn(f64) -> f64>(a: f64, b: f64, func: F) -> f64 {
    // The range is half of the normal range since the five weights add to one
    // (ten weights add to two). The values of the abscissas are multiplied by
    // two to account for this.
    let x_mean = 0.5 * (b + a);
    let x_range = 0.5 * (b - a);

    let mut sum = 0.0;
    for i in 0..5 {
        let dx = x_range * GAUSS_LEGENDRE_ABSCISSAS[i];
        sum += GAUSS_LEGENDRE_WEIGHTS[i] * (func(x_mean + dx) + func(x_mean - dx));
    }

    // Scale the sum to the range of x.
    sum * x_range
}

impl Default for Ellipsoid {
    fn default() -> Self {
        Self::WGS84
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::excessive_precision)]
    fn test_wgs84_radii() {
        let ell = Ellipsoid::WGS84;
        assert_eq!(ell.radii().x, 6378137.0);
        assert_eq!(ell.radii().y, 6378137.0);
        assert!((ell.radii().z - 6356752.3142451793).abs() < 1e-10);
    }

    #[test]
    fn test_geodetic_surface_normal_cartographic() {
        let ell = Ellipsoid::WGS84;
        // At equator, prime meridian: normal should be (1, 0, 0)
        let c = Cartographic::from_radians(0.0, 0.0, 0.0);
        let n = ell.geodetic_surface_normal_cartographic(&c);
        assert!((n.x - 1.0).abs() < 1e-15);
        assert!(n.y.abs() < 1e-15);
        assert!(n.z.abs() < 1e-15);

        // At north pole: normal should be (0, 0, 1)
        let c = Cartographic::from_radians(0.0, std::f64::consts::PI / 2.0, 0.0);
        let n = ell.geodetic_surface_normal_cartographic(&c);
        assert!(n.x.abs() < 1e-15);
        assert!(n.y.abs() < 1e-15);
        assert!((n.z - 1.0).abs() < 1e-15);
    }

    #[test]
    fn test_cartographic_to_cartesian_roundtrip() {
        let ell = Ellipsoid::WGS84;
        let original = Cartographic::from_degrees(21.0, 78.0, 5000.0);
        let cartesian = ell.cartographic_to_cartesian(&original);
        let result = ell.cartesian_to_cartographic(cartesian).unwrap();

        assert!(
            (result.longitude - original.longitude).abs() < 1e-10,
            "longitude diff: {}",
            (result.longitude - original.longitude).abs()
        );
        assert!(
            (result.latitude - original.latitude).abs() < 1e-10,
            "latitude diff: {}",
            (result.latitude - original.latitude).abs()
        );
        assert!(
            (result.height - original.height).abs() < 1e-6,
            "height diff: {}",
            (result.height - original.height).abs()
        );
    }

    #[test]
    fn test_cartographic_to_cartesian_equator() {
        let ell = Ellipsoid::WGS84;
        let c = Cartographic::from_radians(0.0, 0.0, 0.0);
        let cartesian = ell.cartographic_to_cartesian(&c);
        // At equator, prime meridian, height 0: should be (6378137, 0, 0)
        assert!((cartesian.x - 6378137.0).abs() < 1e-6);
        assert!(cartesian.y.abs() < 1e-6);
        assert!(cartesian.z.abs() < 1e-6);
    }

    #[test]
    fn test_scale_to_geodetic_surface() {
        let ell = Ellipsoid::WGS84;
        // A point above the surface should be scaled down to the surface
        let point = DVec3::new(6378137.0 * 2.0, 0.0, 0.0);
        let surface = ell.scale_to_geodetic_surface(point).unwrap();
        assert!((surface.x - 6378137.0).abs() < 1e-6);
        assert!(surface.y.abs() < 1e-6);
        assert!(surface.z.abs() < 1e-6);
    }

    #[test]
    fn test_scale_to_geodetic_surface_center() {
        let ell = Ellipsoid::WGS84;
        // At center, should return None or the center itself
        let result = ell.scale_to_geodetic_surface(DVec3::ZERO);
        // The center is within tolerance, ratio is infinite → None
        assert!(result.is_none());
    }

    #[test]
    fn test_geodetic_surface_normal_at_center() {
        let ell = Ellipsoid::WGS84;
        assert!(ell.geodetic_surface_normal(DVec3::ZERO).is_none());
    }

    #[test]
    fn test_intersection() {
        let ell = Ellipsoid::WGS84;
        // Ray from outside pointing at center along x-axis
        let origin = DVec3::new(6378137.0 * 2.0, 0.0, 0.0);
        let direction = DVec3::new(-1.0, 0.0, 0.0);
        let (t0, t1) = ell.intersection(origin, direction).unwrap();
        // t0 should hit the near surface, t1 the far surface
        let hit0 = origin + direction * t0;
        let hit1 = origin + direction * t1;
        assert!((hit0.x - 6378137.0).abs() < 1e-3);
        assert!((hit1.x + 6378137.0).abs() < 1e-3);
    }

    #[test]
    fn test_multiple_roundtrip_positions() {
        let ell = Ellipsoid::WGS84;
        let test_cases = vec![
            Cartographic::from_degrees(0.0, 0.0, 0.0),
            Cartographic::from_degrees(180.0, 0.0, 0.0),
            Cartographic::from_degrees(-122.4194, 37.7749, 100.0),
            Cartographic::from_degrees(139.6917, 35.6895, 40.0),
            Cartographic::from_degrees(0.0, 89.999, 10000.0),
            Cartographic::from_degrees(-179.999, -89.999, 0.0),
        ];

        for original in &test_cases {
            let cartesian = ell.cartographic_to_cartesian(original);
            let result = ell.cartesian_to_cartographic(cartesian).unwrap();
            assert!(
                (result.longitude - original.longitude).abs() < 1e-10,
                "Failed for {:?}: lon diff = {}",
                original,
                (result.longitude - original.longitude).abs()
            );
            assert!(
                (result.latitude - original.latitude).abs() < 1e-10,
                "Failed for {:?}: lat diff = {}",
                original,
                (result.latitude - original.latitude).abs()
            );
            assert!(
                (result.height - original.height).abs() < 1e-6,
                "Failed for {:?}: height diff = {}",
                original,
                (result.height - original.height).abs()
            );
        }
    }
}
