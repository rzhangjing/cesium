//! Ported from `packages/engine/Source/Core/Stereographic.js`.
//!
//! Represents a point in stereographic coordinates, which can be obtained by
//! projecting a cartesian coordinate from one pole onto a tangent plane at
//! the other pole.
//!
//! DEVIATION: JS stores a reference to a shared frozen
//! `EllipsoidTangentPlane` constant (`NORTH/SOUTH_POLE_TANGENT_PLANE`); the
//! Rust port stores a pole flag and rebuilds the equivalent plane constants
//! on demand.

use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::ellipsoid_tangent_plane::EllipsoidTangentPlane;
use crate::intersection_tests::IntersectionTests;
use crate::math::CesiumMath;
use crate::ray::Ray;
use crate::transforms;

/// Half unit sphere ellipsoid (radii = 0.5, 0.5, 0.5).
pub const HALF_UNIT_SPHERE_RADII: Cartesian3 = Cartesian3 { x: 0.5, y: 0.5, z: 0.5 };

/// North pole position on the half unit sphere.
pub const NORTH_POLE: Cartesian3 = Cartesian3 { x: 0.0, y: 0.0, z: 0.5 };

/// South pole position on the half unit sphere.
pub const SOUTH_POLE: Cartesian3 = Cartesian3 { x: 0.0, y: 0.0, z: -0.5 };

/// Builds the `HALF_UNIT_SPHERE` ellipsoid constant.
pub fn half_unit_sphere() -> Ellipsoid {
    Ellipsoid::new(0.5, 0.5, 0.5)
}

/// Builds the equivalent of the frozen JS
/// `Stereographic.NORTH_POLE_TANGENT_PLANE` /
/// `Stereographic.SOUTH_POLE_TANGENT_PLANE` constants.
pub fn pole_tangent_plane(north: bool) -> EllipsoidTangentPlane {
    let origin = if north { NORTH_POLE } else { SOUTH_POLE };
    let ellipsoid = half_unit_sphere();
    let transform = transforms::east_north_up_to_fixed_frame_new(&origin, Some(&ellipsoid));
    EllipsoidTangentPlane::from_transform4(&transform, Some(ellipsoid))
}

/// Represents a point in stereographic coordinates, which can be obtained by
/// projecting a cartesian coordinate from one pole onto a tangent plane at
/// the other pole. The stereographic projection faithfully represents the
/// relative directions of all great circles passing through its center
/// point. To faithfully represent angles everywhere, this is a conformal
/// projection, which means points are projected onto an arbitrary sphere.
#[derive(Clone, Debug)]
pub struct Stereographic {
    /// The stereographic position (2D coordinates).
    pub position: Cartesian2,
    /// True when projected onto the north pole tangent plane (JS
    /// `tangentPlane === Stereographic.NORTH_POLE_TANGENT_PLANE`).
    north_pole: bool,
}

impl Default for Stereographic {
    fn default() -> Self {
        Self {
            position: Cartesian2::ZERO,
            north_pole: true,
        }
    }
}

impl Stereographic {
    /// Creates a new Stereographic instance.
    pub fn new(position: Option<Cartesian2>) -> Self {
        Self {
            position: position.unwrap_or(Cartesian2::ZERO),
            north_pole: true,
        }
    }

    /// Gets the x coordinate.
    pub fn x(&self) -> f64 {
        self.position.x
    }

    /// Gets the y coordinate.
    pub fn y(&self) -> f64 {
        self.position.y
    }

    /// True when this point uses the north pole tangent plane.
    pub fn is_north_pole(&self) -> bool {
        self.north_pole
    }

    /// Gets the ellipsoid of the associated tangent plane.
    pub fn ellipsoid(&self) -> Ellipsoid {
        half_unit_sphere()
    }

    /// Gets the associated tangent plane.
    pub fn tangent_plane(&self) -> EllipsoidTangentPlane {
        pole_tangent_plane(self.north_pole)
    }

    /// Computes the conformal latitude, or the ellipsoidal latitude projected
    /// onto an arbitrary sphere.
    pub fn conformal_latitude(&self) -> f64 {
        let r = Cartesian2::magnitude(&self.position);
        let d = 2.0 * self.ellipsoid().maximum_radius();
        let sign = if self.north_pole { 1.0 } else { -1.0 };
        sign * (CesiumMath::PI_OVER_TWO - 2.0 * f64::atan2(r, d))
    }

    /// Computes the longitude.
    pub fn longitude(&self) -> f64 {
        let mut longitude = CesiumMath::PI_OVER_TWO + f64::atan2(self.position.y, self.position.x);
        if longitude > std::f64::consts::PI {
            longitude -= CesiumMath::TWO_PI;
        }
        longitude
    }

    /// Computes the latitude based on an ellipsoid (JS
    /// `Stereographic.prototype.getLatitude`).
    pub fn get_latitude(&self, ellipsoid: Option<&Ellipsoid>) -> f64 {
        let ellipsoid = ellipsoid.copied().unwrap_or(Ellipsoid::WGS84);
        let plane_ellipsoid = self.ellipsoid();
        let mut carto = Cartographic::new(self.longitude(), self.conformal_latitude(), 0.0);
        let mut cartesian = Cartesian3::default();
        plane_ellipsoid.cartographic_to_cartesian(&carto, &mut cartesian);
        ellipsoid.cartesian_to_cartographic(&cartesian, &mut carto);
        carto.latitude
    }

    /// Computes the projection of the provided 3D position onto the 2D polar
    /// plane, radially outward from the opposite pole.
    pub fn from_cartesian(cartesian: &Cartesian3, result: Option<&mut Self>) -> Self {
        let sign = CesiumMath::sign_not_zero(cartesian.z);
        let north = sign >= 0.0;
        let tangent_plane = pole_tangent_plane(north);
        let origin = if north { SOUTH_POLE } else { NORTH_POLE };

        let mut ray_origin = Cartesian3::default();
        tangent_plane
            .ellipsoid()
            .scale_to_geocentric_surface(cartesian, &mut ray_origin);
        let mut direction = Cartesian3::subtract_new(&ray_origin, &origin);
        let mut normalized = Cartesian3::default();
        Cartesian3::normalize(&direction, &mut normalized);
        direction = normalized;

        let ray = Ray::new(Some(&ray_origin), Some(&direction));
        let mut intersection_point =
            IntersectionTests::ray_plane(&ray, tangent_plane.plane()).unwrap_or_default();
        intersection_point = Cartesian3::subtract_new(&intersection_point, &origin);
        let x = Cartesian3::dot(tangent_plane.x_axis(), &intersection_point);
        let y = sign * Cartesian3::dot(tangent_plane.y_axis(), &intersection_point);

        match result {
            None => Self {
                position: Cartesian2::new(x, y),
                north_pole: north,
            },
            Some(r) => {
                r.position = Cartesian2::new(x, y);
                r.north_pole = north;
                r.clone()
            }
        }
    }

    /// Computes the projection of the provided 3D positions onto the 2D
    /// polar plane.
    pub fn from_cartesian_array(cartesians: &[Cartesian3], result: Option<Vec<Self>>) -> Vec<Self> {
        let mut result = result.unwrap_or_default();
        result.resize_with(cartesians.len(), Self::default);
        for (i, cartesian) in cartesians.iter().enumerate() {
            result[i] = Self::from_cartesian(cartesian, Some(&mut result[i]));
        }
        result
    }

    /// Duplicates a Stereographic instance (JS `Stereographic.clone`).
    pub fn clone_stereographic(stereographic: Option<&Self>, result: Option<&mut Self>) -> Option<Self> {
        let stereographic = stereographic?;
        match result {
            None => Some(stereographic.clone()),
            Some(r) => {
                r.position = stereographic.position;
                r.north_pole = stereographic.north_pole;
                Some(r.clone())
            }
        }
    }
}
