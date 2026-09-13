//! Stereographic projection coordinates.
//! Maps to CesiumJS `Core/Stereographic.js`

use crate::ellipsoid::Ellipsoid;
use crate::ellipsoid_tangent_plane::EllipsoidTangentPlane;
use crate::math_utils;
use crate::ray::{ray_plane, Ray};
use glam::{DVec2, DVec3};

/// An ellipsoid with radii (0.5, 0.5, 0.5).
pub const HALF_UNIT_SPHERE: Ellipsoid = Ellipsoid::from_radii_unchecked(0.5, 0.5, 0.5);

/// North pole on the half-unit sphere.
pub const NORTH_POLE: DVec3 = DVec3::new(0.0, 0.0, 0.5);
/// South pole on the half-unit sphere.
pub const SOUTH_POLE: DVec3 = DVec3::new(0.0, 0.0, -0.5);

/// Identifies which pole tangent plane is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoleTangentPlane {
    North,
    South,
}

/// Represents a point in stereographic coordinates, obtained by projecting
/// a cartesian coordinate from one pole onto a tangent plane at the other pole.
///
/// Maps to CesiumJS `Stereographic`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stereographic {
    /// The stereographic 2D coordinates.
    pub position: DVec2,
    /// Which pole tangent plane was used.
    pub tangent_plane: PoleTangentPlane,
}

impl Default for Stereographic {
    fn default() -> Self {
        Self {
            position: DVec2::ZERO,
            tangent_plane: PoleTangentPlane::North,
        }
    }
}

impl Stereographic {
    /// Creates a new Stereographic with the given position and tangent plane.
    pub fn new(position: DVec2, tangent_plane: PoleTangentPlane) -> Self {
        Self {
            position,
            tangent_plane,
        }
    }

    /// Gets the x coordinate.
    #[inline]
    pub fn x(&self) -> f64 {
        self.position.x
    }

    /// Gets the y coordinate.
    #[inline]
    pub fn y(&self) -> f64 {
        self.position.y
    }

    /// Gets the ellipsoid (always the half-unit sphere).
    #[inline]
    pub fn ellipsoid(&self) -> &'static Ellipsoid {
        &HALF_UNIT_SPHERE
    }

    /// Computes the conformal latitude (ellipsoidal latitude projected onto an arbitrary sphere).
    pub fn conformal_latitude(&self) -> f64 {
        let r = self.position.length();
        let d = 2.0 * HALF_UNIT_SPHERE.maximum_radius();
        let sign = match self.tangent_plane {
            PoleTangentPlane::North => 1.0,
            PoleTangentPlane::South => -1.0,
        };
        sign * (math_utils::PI_OVER_TWO - 2.0 * r.atan2(d))
    }

    /// Computes the longitude.
    pub fn longitude(&self) -> f64 {
        let mut longitude = math_utils::PI_OVER_TWO + self.position.y.atan2(self.position.x);
        if longitude > std::f64::consts::PI {
            longitude -= math_utils::TWO_PI;
        }
        longitude
    }

    /// Computes the geodetic latitude on the given ellipsoid.
    ///
    /// Maps to `Stereographic.prototype.getLatitude`
    pub fn get_latitude(&self, ellipsoid: &Ellipsoid) -> f64 {
        let conformal_lat = self.conformal_latitude();
        let longitude = self.longitude();

        // Convert conformal latitude on half-unit sphere to cartesian
        let cos_lat = conformal_lat.cos();
        let cartesian = DVec3::new(
            HALF_UNIT_SPHERE.maximum_radius() * cos_lat * longitude.cos(),
            HALF_UNIT_SPHERE.maximum_radius() * cos_lat * longitude.sin(),
            HALF_UNIT_SPHERE.maximum_radius() * conformal_lat.sin(),
        );

        // Convert that cartesian to cartographic on the target ellipsoid
        ellipsoid
            .cartesian_to_cartographic(cartesian)
            .map(|c| c.latitude)
            .unwrap_or(conformal_lat)
    }

    /// Computes the projection of the provided 3D position onto the 2D polar plane.
    ///
    /// Maps to `Stereographic.fromCartesian`
    pub fn from_cartesian(cartesian: DVec3) -> Self {
        let sign = if cartesian.z >= 0.0 { 1.0 } else { -1.0 };

        let (tangent_plane_id, origin) = if sign < 0.0 {
            (PoleTangentPlane::South, NORTH_POLE)
        } else {
            (PoleTangentPlane::North, SOUTH_POLE)
        };

        let tangent_plane = Self::get_tangent_plane(tangent_plane_id);

        // Ray from geocentric surface point toward the opposite pole
        let surface_point = HALF_UNIT_SPHERE
            .scale_to_geocentric_surface(cartesian)
            .unwrap_or(cartesian);
        let direction = (surface_point - origin).normalize();
        let ray = Ray {
            origin: surface_point,
            direction,
        };

        let intersection_point = ray_plane(&ray, tangent_plane.plane())
            .expect("ray must intersect tangent plane");

        let v = intersection_point - origin;
        let x = tangent_plane.x_axis().dot(v);
        let y = sign * tangent_plane.y_axis().dot(v);

        Self {
            position: DVec2::new(x, y),
            tangent_plane: tangent_plane_id,
        }
    }

    /// Computes the projection of an array of 3D positions.
    ///
    /// Maps to `Stereographic.fromCartesianArray`
    pub fn from_cartesian_array(cartesians: &[DVec3]) -> Vec<Self> {
        cartesians.iter().map(|&c| Self::from_cartesian(c)).collect()
    }

    /// Gets the tangent plane for the given pole.
    fn get_tangent_plane(pole: PoleTangentPlane) -> EllipsoidTangentPlane {
        match pole {
            PoleTangentPlane::North => EllipsoidTangentPlane::new(NORTH_POLE, &HALF_UNIT_SPHERE),
            PoleTangentPlane::South => EllipsoidTangentPlane::new(SOUTH_POLE, &HALF_UNIT_SPHERE),
        }
    }
}
