//! Ported from `packages/engine/Source/Core/EllipsoidTangentPlane.js`.
//!
//! A plane tangent to an ellipsoid at a given origin point.

use crate::axis_aligned_bounding_box::AxisAlignedBoundingBox;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;
use crate::intersection_tests::IntersectionTests;
use crate::matrix4::Matrix4;
use crate::plane::Plane;
use crate::ray::Ray;
use crate::transforms;

/// A plane tangent to the provided ellipsoid at the provided origin.
pub struct EllipsoidTangentPlane {
    ellipsoid: Ellipsoid,
    origin: Cartesian3,
    x_axis: Cartesian3,
    y_axis: Cartesian3,
    plane: Plane,
}

impl EllipsoidTangentPlane {
    /// Port of the `EllipsoidTangentPlane` constructor.
    ///
    /// DEVIATION: JS throws a DeveloperError when the origin is at the center
    /// of the ellipsoid (`scaleToGeodeticSurface` fails); this port returns
    /// `None` in that case (callers such as `OrientedBoundingBox` and
    /// `PolylineVolumeGeometryLibrary` already handle the `Option`).
    pub fn new(origin: &Cartesian3, ellipsoid: Option<Ellipsoid>) -> Option<Self> {
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);
        let mut surface_point = Cartesian3::default();
        if !ellipsoid.scale_to_geodetic_surface(origin, &mut surface_point) {
            return None;
        }

        let transform =
            transforms::east_north_up_to_fixed_frame_new(&surface_point, Some(&ellipsoid));
        Some(Self::from_transform4(&transform, Some(ellipsoid)))
    }

    /// Port of `EllipsoidTangentPlane.fromTransformation`.
    pub fn from_transform4(transform: &Matrix4, ellipsoid: Option<Ellipsoid>) -> Self {
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

    /// Port of `EllipsoidTangentPlane.fromPoints`.
    ///
    /// Creates a plane tangent to the ellipsoid at the center of the
    /// axis-aligned bounding box of the provided positions.
    pub fn from_points(cartesians: &[Cartesian3], ellipsoid: Option<Ellipsoid>) -> Option<Self> {
        let center = AxisAlignedBoundingBox::from_points(Some(cartesians)).center;
        Self::new(&center, ellipsoid)
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

    /// Port of `EllipsoidTangentPlane.prototype.projectPointOntoPlane`.
    ///
    /// Computes the position of the projection of the point onto the plane;
    /// returns `None` when the ray from the point through the ellipsoid
    /// center never hits the plane (both directions tried).
    pub fn project_point_onto_plane(&self, cartesian: &Cartesian3) -> Option<Cartesian2> {
        let mut direction = Cartesian3::normalize_new(cartesian);
        let ray = Ray::new(Some(cartesian), Some(&direction));
        let mut intersection_point = IntersectionTests::ray_plane(&ray, &self.plane);
        if intersection_point.is_none() {
            direction = Cartesian3::negate_new(&direction);
            let ray = Ray::new(Some(cartesian), Some(&direction));
            intersection_point = IntersectionTests::ray_plane(&ray, &self.plane);
        }
        let intersection_point = intersection_point?;

        let v = Cartesian3::subtract_new(&intersection_point, &self.origin);
        let x = Cartesian3::dot(&self.x_axis, &v);
        let y = Cartesian3::dot(&self.y_axis, &v);
        Some(Cartesian2::from_elements_new(x, y))
    }

    /// Port of `EllipsoidTangentPlane.prototype.projectPointsOntoPlane`.
    ///
    /// Projects each point onto the plane, skipping points that cannot be
    /// projected (JS `result.length = count` truncation).
    pub fn project_points_onto_plane(&self, cartesians: &[Cartesian3]) -> Vec<Cartesian2> {
        let mut result = Vec::with_capacity(cartesians.len());
        for cartesian in cartesians {
            if let Some(p) = self.project_point_onto_plane(cartesian) {
                result.push(p);
            }
        }
        result
    }

    /// Port of `EllipsoidTangentPlane.prototype.projectPointToNearestOnPlane`.
    ///
    /// Projects a 3D point onto the tangent plane as a 2D coordinate along
    /// the plane normal.
    pub fn project_point_to_nearest_tangent_plane(&self, cartesian: &Cartesian3) -> Cartesian2 {
        let diff = Cartesian3::subtract_new(cartesian, &self.origin);
        let x = Cartesian3::dot(&diff, &self.x_axis);
        let y = Cartesian3::dot(&diff, &self.y_axis);
        Cartesian2::from_elements_new(x, y)
    }
}
