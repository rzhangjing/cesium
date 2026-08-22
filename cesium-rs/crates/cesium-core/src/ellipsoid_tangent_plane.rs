//! Ported from `packages/engine/Source/Core/EllipsoidTangentPlane.js`.
//!
//! A plane tangent to an ellipsoid at a given origin point.

use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;
use crate::matrix4::Matrix4;
use crate::plane::Plane;

/// A plane tangent to the provided ellipsoid at the provided origin.
pub struct EllipsoidTangentPlane {
    ellipsoid: Ellipsoid,
    origin: Cartesian3,
    x_axis: Cartesian3,
    y_axis: Cartesian3,
    plane: Plane,
}

impl EllipsoidTangentPlane {
    /// Creates a new EllipsoidTangentPlane.
    pub fn new(origin: &Cartesian3, ellipsoid: Option<Ellipsoid>) -> Option<Self> {
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);
        let mut surface_point = Cartesian3::default();
        if !ellipsoid.scale_to_geodetic_surface(origin, &mut surface_point) {
            return None;
        }

        // Compute east-north-up frame at origin
        let mut up = Cartesian3::default();
        ellipsoid.geodetic_surface_normal(&surface_point, &mut up);

        let east = Cartesian3::new(-surface_point.y, surface_point.x, 0.0);
        let east_mag = Cartesian3::magnitude(&east);
        let east = if east_mag < 1e-10 {
            Cartesian3::new(-1.0, 0.0, 0.0)
        } else {
            Cartesian3::multiply_by_scalar_new(&east, 1.0 / east_mag)
        };
        let north = Cartesian3::cross_new(&up, &east);

        let plane = Plane::from_point_normal_new(&surface_point, &up);

        Some(Self {
            ellipsoid,
            origin: surface_point,
            x_axis: east,
            y_axis: north,
            plane,
        })
    }

    /// Creates from a 4x4 transformation matrix (east-north-up frame).
    pub fn from_transform4(
        transform: &Matrix4,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);
        let origin = Matrix4::get_translation_new(transform);

        let col0 = Matrix4::get_column_new(transform, 0);
        let col1 = Matrix4::get_column_new(transform, 1);
        let col2 = Matrix4::get_column_new(transform, 2);

        let x_axis = Cartesian3::new(col0.x, col0.y, col0.z);
        let y_axis = Cartesian3::new(col1.x, col1.y, col1.z);
        let normal = Cartesian3::new(col2.x, col2.y, col2.z);

        let plane = Plane::from_point_normal_new(&origin, &normal);

        Self {
            ellipsoid,
            origin,
            x_axis,
            y_axis,
            plane,
        }
    }

    /// Gets the ellipsoid.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// Gets the origin.
    pub fn origin(&self) -> &Cartesian3 {
        &self.origin
    }

    /// Gets the x-axis (east direction).
    pub fn x_axis(&self) -> &Cartesian3 {
        &self.x_axis
    }

    /// Gets the y-axis (north direction).
    pub fn y_axis(&self) -> &Cartesian3 {
        &self.y_axis
    }

    /// Gets the plane.
    pub fn plane(&self) -> &Plane {
        &self.plane
    }

    /// Projects a 3D point onto the tangent plane as a 2D coordinate.
    pub fn project_point_to_nearest_tangent_plane(
        &self,
        cartesian: &Cartesian3,
    ) -> Cartesian2 {
        let diff = Cartesian3::subtract_new(cartesian, &self.origin);
        let x = Cartesian3::dot(&diff, &self.x_axis);
        let y = Cartesian3::dot(&diff, &self.y_axis);
        Cartesian2::from_elements_new(x, y)
    }
}
