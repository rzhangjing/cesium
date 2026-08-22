//! Ported from packages/engine/Source/Core/Ellipsoid.js
//!
//! A quadratic surface defined in Cartesian coordinates by the equation
//! `(x / a)^2 + (y / b)^2 + (z / c)^2 = 1`. Primarily used to represent
//! the shape of planetary bodies.

use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::{Cartographic, EllipsoidParams};
use crate::math::CesiumMath;
use crate::scale_to_geodetic_surface::scale_to_geodetic_surface;

/// Internal precomputed data for an Ellipsoid.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct EllipsoidData {
    radii: Cartesian3,
    radii_squared: Cartesian3,
    radii_to_the_fourth: Cartesian3,
    one_over_radii: Cartesian3,
    one_over_radii_squared: Cartesian3,
    minimum_radius: f64,
    maximum_radius: f64,
    center_tolerance_squared: f64,
    squared_x_over_squared_z: f64,
}

const fn const_init(x: f64, y: f64, z: f64) -> EllipsoidData {
    let xx = x * x;
    let yy = y * y;
    let zz = z * z;
    let oox = if x == 0.0 { 0.0 } else { 1.0 / x };
    let ooy = if y == 0.0 { 0.0 } else { 1.0 / y };
    let ooz = if z == 0.0 { 0.0 } else { 1.0 / z };
    let ooxx = if x == 0.0 { 0.0 } else { 1.0 / xx };
    let ooyy = if y == 0.0 { 0.0 } else { 1.0 / yy };
    let oozz = if z == 0.0 { 0.0 } else { 1.0 / zz };
    let min_r = if x < y { if x < z { x } else { z } } else { if y < z { y } else { z } };
    let max_r = if x > y { if x > z { x } else { z } } else { if y > z { y } else { z } };
    let sqx_over_sqz = if zz != 0.0 { xx / zz } else { 0.0 };
    EllipsoidData {
        radii: Cartesian3::new(x, y, z),
        radii_squared: Cartesian3::new(xx, yy, zz),
        radii_to_the_fourth: Cartesian3::new(xx * xx, yy * yy, zz * zz),
        one_over_radii: Cartesian3::new(oox, ooy, ooz),
        one_over_radii_squared: Cartesian3::new(ooxx, ooyy, oozz),
        minimum_radius: min_r,
        maximum_radius: max_r,
        center_tolerance_squared: CesiumMath::EPSILON1,
        squared_x_over_squared_z: sqx_over_sqz,
    }
}

fn initialize(radii: &Cartesian3) -> EllipsoidData {
    const_init(radii.x, radii.y, radii.z)
}

/// A quadratic surface defined in Cartesian coordinates by the equation
/// `(x / a)^2 + (y / b)^2 + (z / c)^2 = 1`.
///
/// Port of `Ellipsoid`.
#[derive(Clone, Copy, Debug)]
pub struct Ellipsoid {
    data: EllipsoidData,
}

impl Ellipsoid {
    /// Creates a new Ellipsoid with the provided radii.
    ///
    /// Port of the `Ellipsoid(x, y, z)` constructor.
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            data: initialize(&Cartesian3::new(x, y, z)),
        }
    }

    /// An Ellipsoid instance initialized to the WGS84 standard.
    pub const WGS84: Ellipsoid = Ellipsoid {
        data: const_init(6378137.0, 6378137.0, 6356752.3142451793),
    };

    /// An Ellipsoid instance initialized to radii of (1.0, 1.0, 1.0).
    pub const UNIT_SPHERE: Ellipsoid = Ellipsoid {
        data: const_init(1.0, 1.0, 1.0),
    };

    /// An Ellipsoid instance initialized to a sphere with the lunar radius.
    pub const MOON: Ellipsoid = Ellipsoid {
        data: const_init(
            CesiumMath::LUNAR_RADIUS,
            CesiumMath::LUNAR_RADIUS,
            CesiumMath::LUNAR_RADIUS,
        ),
    };

    /// An Ellipsoid instance initialized to the mean radii of Mars.
    pub const MARS: Ellipsoid = Ellipsoid {
        data: const_init(3396190.0, 3396190.0, 3376200.0),
    };

    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize = Cartesian3::PACKED_LENGTH;

    // --- Properties ---

    pub fn radii(&self) -> &Cartesian3 { &self.data.radii }
    pub fn radii_squared(&self) -> &Cartesian3 { &self.data.radii_squared }
    pub fn radii_to_the_fourth(&self) -> &Cartesian3 { &self.data.radii_to_the_fourth }
    pub fn one_over_radii(&self) -> &Cartesian3 { &self.data.one_over_radii }
    pub fn one_over_radii_squared(&self) -> &Cartesian3 { &self.data.one_over_radii_squared }
    pub fn minimum_radius(&self) -> f64 { self.data.minimum_radius }
    pub fn maximum_radius(&self) -> f64 { self.data.maximum_radius }

    // --- Static methods ---

    /// Computes an Ellipsoid from a Cartesian specifying the radii.
    ///
    /// Port of `Ellipsoid.fromCartesian3`.
    pub fn from_cartesian3(cartesian: Option<&Cartesian3>) -> Self {
        match cartesian {
            Some(c) => Self { data: initialize(c) },
            None => Self { data: initialize(&Cartesian3::ZERO) },
        }
    }

    /// Duplicates an Ellipsoid instance.
    pub fn clone_ellipsoid(ellipsoid: &Self) -> Self {
        *ellipsoid
    }

    /// Port of `Ellipsoid.pack`.
    pub fn pack(value: &Self, array: &mut [f64], starting_index: Option<usize>) {
        Cartesian3::pack(&value.data.radii, array, starting_index);
    }

    /// Port of `Ellipsoid.unpack`.
    pub fn unpack(array: &[f64], starting_index: Option<usize>) -> Self {
        let radii = Cartesian3::unpack_new(array, starting_index);
        Self::from_cartesian3(Some(&radii))
    }

    // --- Instance methods ---

    /// Computes the unit vector directed from the center toward the provided
    /// Cartesian position (= `Cartesian3.normalize`).
    pub fn geocentric_surface_normal(cartesian: &Cartesian3, result: &mut Cartesian3) {
        Cartesian3::normalize(cartesian, result);
    }

    /// Port of `Ellipsoid#geodeticSurfaceNormalCartographic`.
    pub fn geodetic_surface_normal_cartographic(
        &self,
        cartographic: &Cartographic,
        result: &mut Cartesian3,
    ) {
        let lon = cartographic.longitude;
        let lat = cartographic.latitude;
        let cos_lat = lat.cos();

        result.x = cos_lat * lon.cos();
        result.y = cos_lat * lon.sin();
        result.z = lat.sin();

        let n = Cartesian3::normalize_new(result);
        *result = n;
    }

    /// Port of `Ellipsoid#geodeticSurfaceNormal`.
    /// Returns `false` if the cartesian is at the center (JS returns `undefined`).
    pub fn geodetic_surface_normal(
        &self,
        cartesian: &Cartesian3,
        result: &mut Cartesian3,
    ) -> bool {
        if Cartesian3::equals_epsilon(Some(cartesian), Some(&Cartesian3::ZERO), Some(CesiumMath::EPSILON14), None) {
            return false;
        }
        Cartesian3::multiply_components(cartesian, &self.data.one_over_radii_squared, result);
        let n = Cartesian3::normalize_new(result);
        *result = n;
        true
    }

    /// Port of `Ellipsoid#cartographicToCartesian`.
    pub fn cartographic_to_cartesian(
        &self,
        cartographic: &Cartographic,
        result: &mut Cartesian3,
    ) {
        let mut n = Cartesian3::default();
        self.geodetic_surface_normal_cartographic(cartographic, &mut n);

        let k = Cartesian3::multiply_components_new(&self.data.radii_squared, &n);
        let gamma = Cartesian3::dot(&n, &k).sqrt();
        let k = Cartesian3::divide_by_scalar_new(&k, gamma);
        let n_offset = Cartesian3::multiply_by_scalar_new(&n, cartographic.height);

        Cartesian3::add(&k, &n_offset, result);
    }

    /// Port of `Ellipsoid#cartesianToCartographic`.
    /// Returns `false` if the cartesian is at the center (JS returns `undefined`).
    pub fn cartesian_to_cartographic(
        &self,
        cartesian: &Cartesian3,
        result: &mut Cartographic,
    ) -> bool {
        let mut p = Cartesian3::default();
        if !scale_to_geodetic_surface(
            cartesian,
            &self.data.one_over_radii,
            &self.data.one_over_radii_squared,
            self.data.center_tolerance_squared,
            &mut p,
        ) {
            return false;
        }

        let mut n = Cartesian3::default();
        if !self.geodetic_surface_normal(&p, &mut n) {
            return false;
        }

        let h = Cartesian3::subtract_new(cartesian, &p);
        result.longitude = n.y.atan2(n.x);
        result.latitude = n.z.asin();
        result.height = CesiumMath::sign(Cartesian3::dot(&h, cartesian)) * Cartesian3::magnitude(&h);
        true
    }

    /// Port of `Ellipsoid#scaleToGeodeticSurface`.
    /// Returns `false` if the position is at the center.
    pub fn scale_to_geodetic_surface(
        &self,
        cartesian: &Cartesian3,
        result: &mut Cartesian3,
    ) -> bool {
        scale_to_geodetic_surface(
            cartesian,
            &self.data.one_over_radii,
            &self.data.one_over_radii_squared,
            self.data.center_tolerance_squared,
            result,
        )
    }

    /// Port of `Ellipsoid#scaleToGeocentricSurface`.
    pub fn scale_to_geocentric_surface(
        &self,
        cartesian: &Cartesian3,
        result: &mut Cartesian3,
    ) {
        let one_over_radii_squared = &self.data.one_over_radii_squared;
        let beta = 1.0 / (cartesian.x * cartesian.x * one_over_radii_squared.x
            + cartesian.y * cartesian.y * one_over_radii_squared.y
            + cartesian.z * cartesian.z * one_over_radii_squared.z)
            .sqrt();
        Cartesian3::multiply_by_scalar(cartesian, beta, result);
    }

    /// Port of `Ellipsoid#transformPositionToScaledSpace`.
    pub fn transform_position_to_scaled_space(
        &self,
        position: &Cartesian3,
        result: &mut Cartesian3,
    ) {
        Cartesian3::multiply_components(position, &self.data.one_over_radii, result);
    }

    /// Port of `Ellipsoid#transformPositionFromScaledSpace`.
    pub fn transform_position_from_scaled_space(
        &self,
        position: &Cartesian3,
        result: &mut Cartesian3,
    ) {
        Cartesian3::multiply_components(position, &self.data.radii, result);
    }

    /// Port of `Ellipsoid#equals`.
    pub fn equals(&self, right: &Self) -> bool {
        Cartesian3::equals(Some(&self.data.radii), Some(&right.data.radii))
    }

    /// Port of `Ellipsoid#toString`.
    pub fn to_string_repr(&self) -> String {
        format!("{}", self.data.radii)
    }

    /// Port of `Ellipsoid#getSurfaceNormalIntersectionWithZAxis`.
    /// Returns `false` if the intersection is outside the ellipsoid.
    pub fn get_surface_normal_intersection_with_z_axis(
        &self,
        position: &Cartesian3,
        buffer: Option<f64>,
        result: &mut Cartesian3,
    ) -> bool {
        let buf = buffer.unwrap_or(0.0);
        let squared_x_over_squared_z = if self.data.radii_squared.z != 0.0 {
            self.data.radii_squared.x / self.data.radii_squared.z
        } else {
            0.0
        };

        result.x = 0.0;
        result.y = 0.0;
        result.z = position.z * (1.0 - squared_x_over_squared_z);

        if result.z.abs() >= self.data.radii.z - buf {
            return false;
        }
        true
    }

    /// Port of `Ellipsoid#getLocalCurvature`.
    pub fn get_local_curvature(
        &self,
        surface_position: &Cartesian3,
        result: &mut Cartesian2,
    ) {
        let mut endpoint = Cartesian3::default();
        self.get_surface_normal_intersection_with_z_axis(
            surface_position,
            Some(0.0),
            &mut endpoint,
        );

        let prime_vertical_radius = Cartesian3::distance(surface_position, &endpoint);
        let max_r_sq = self.data.maximum_radius * self.data.maximum_radius;
        let radius_ratio = (self.data.minimum_radius * prime_vertical_radius) / max_r_sq;
        let meridional_radius = prime_vertical_radius * radius_ratio * radius_ratio;

        result.x = 1.0 / prime_vertical_radius;
        result.y = 1.0 / meridional_radius;
    }

    /// Provides the [`EllipsoidParams`] for use with [`Cartographic::from_cartesian`].
    pub fn ellipsoid_params(&self) -> EllipsoidParams {
        EllipsoidParams {
            one_over_radii: self.data.one_over_radii,
            one_over_radii_squared: self.data.one_over_radii_squared,
            center_tolerance_squared: self.data.center_tolerance_squared,
        }
    }
}

impl PartialEq for Ellipsoid {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl std::fmt::Display for Ellipsoid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data.radii)
    }
}
