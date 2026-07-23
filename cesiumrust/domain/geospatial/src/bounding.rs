//! Bounding volumes - BoundingSphere, OrientedBoundingBox, AxisAlignedBoundingBox.
//! Maps to CesiumJS `Core/BoundingSphere.js`, `Core/OrientedBoundingBox.js`, `Core/AxisAlignedBoundingBox.js`

use crate::ray::Intersect;
use glam::{DMat3, DVec3};
use serde::{Deserialize, Serialize};

/// A bounding sphere defined by a center point and radius.
/// Maps to CesiumJS `BoundingSphere`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingSphere {
    /// The center of the sphere.
    pub center: DVec3,
    /// The radius of the sphere.
    pub radius: f64,
}

impl BoundingSphere {
    pub fn new(center: DVec3, radius: f64) -> Self {
        Self { center, radius }
    }

    /// Creates a bounding sphere from a set of points.
    /// Maps to `BoundingSphere.fromPoints`
    pub fn from_points(points: &[DVec3]) -> Self {
        if points.is_empty() {
            return Self {
                center: DVec3::ZERO,
                radius: 0.0,
            };
        }

        // Compute centroid
        let mut center = DVec3::ZERO;
        for p in points {
            center += *p;
        }
        center /= points.len() as f64;

        // Compute maximum distance from center
        let mut radius_sq: f64 = 0.0;
        for p in points {
            let dist_sq = (p - center).length_squared();
            if dist_sq > radius_sq {
                radius_sq = dist_sq;
            }
        }

        Self {
            center,
            radius: radius_sq.sqrt(),
        }
    }

    /// Computes the distance from the closest point on the sphere to a point.
    /// Maps to `BoundingSphere.distanceTo`
    pub fn distance_to(&self, point: DVec3) -> f64 {
        let dist = (point - self.center).length();
        (dist - self.radius).max(0.0)
    }

    /// Determines if a point is inside the sphere.
    pub fn contains(&self, point: DVec3) -> bool {
        (point - self.center).length_squared() <= self.radius * self.radius
    }

    /// Computes the union of two bounding spheres.
    /// Maps to `BoundingSphere.union`
    pub fn union(&self, other: &Self) -> Self {
        let diff = other.center - self.center;
        let dist = diff.length();

        // One sphere contains the other
        if dist + other.radius <= self.radius {
            return *self;
        }
        if dist + self.radius <= other.radius {
            return *other;
        }

        let new_radius = (dist + self.radius + other.radius) * 0.5;
        let new_center = self.center + diff * ((new_radius - self.radius) / dist);

        Self {
            center: new_center,
            radius: new_radius,
        }
    }

    /// Expands the sphere by a given amount.
    pub fn expand(&self, amount: f64) -> Self {
        Self {
            center: self.center,
            radius: self.radius + amount,
        }
    }

    /// Transforms the sphere by a 4x4 matrix.
    pub fn transform(&self, matrix: &glam::DMat4) -> Self {
        let center = matrix.transform_point3(self.center);
        // Scale the radius by the maximum scale factor of the matrix
        let scale_x = matrix.x_axis.truncate().length();
        let scale_y = matrix.y_axis.truncate().length();
        let scale_z = matrix.z_axis.truncate().length();
        let max_scale = scale_x.max(scale_y).max(scale_z);
        Self {
            center,
            radius: self.radius * max_scale,
        }
    }
}

/// An oriented bounding box defined by a center and half-axes.
/// Maps to CesiumJS `OrientedBoundingBox`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OrientedBoundingBox {
    /// The center of the box.
    pub center: DVec3,
    /// The three half-axis vectors (columns define the box orientation and size).
    pub half_axes: DMat3,
}

impl OrientedBoundingBox {
    pub fn new(center: DVec3, half_axes: DMat3) -> Self {
        Self { center, half_axes }
    }

    /// Creates an OBB from center, direction axes, and half-lengths.
    pub fn from_axes_half_lengths(
        center: DVec3,
        u_axis: DVec3,
        v_axis: DVec3,
        w_axis: DVec3,
        half_u: f64,
        half_v: f64,
        half_w: f64,
    ) -> Self {
        let half_axes = DMat3::from_cols(
            u_axis * half_u,
            v_axis * half_v,
            w_axis * half_w,
        );
        Self { center, half_axes }
    }

    /// Computes the distance from the closest point on the OBB to a point.
    /// Maps to `OrientedBoundingBox.distanceTo`
    pub fn distance_to(&self, point: DVec3) -> f64 {
        let offset = point - self.center;

        let u = self.half_axes.x_axis;
        let v = self.half_axes.y_axis;
        let w = self.half_axes.z_axis;

        let u_half = u.length();
        let v_half = v.length();
        let w_half = w.length();

        let u_dir = if u_half > 0.0 { u / u_half } else { DVec3::X };
        let v_dir = if v_half > 0.0 { v / v_half } else { DVec3::Y };
        let w_dir = if w_half > 0.0 { w / w_half } else { DVec3::Z };

        let d_u = offset.dot(u_dir).abs() - u_half;
        let d_v = offset.dot(v_dir).abs() - v_half;
        let d_w = offset.dot(w_dir).abs() - w_half;

        let outside = DVec3::new(d_u.max(0.0), d_v.max(0.0), d_w.max(0.0));
        outside.length()
    }

    /// Converts this OBB to a bounding sphere.
    pub fn to_bounding_sphere(&self) -> BoundingSphere {
        let radius = self.half_axes.x_axis.length().max(
            self.half_axes.y_axis.length().max(self.half_axes.z_axis.length()),
        );
        BoundingSphere {
            center: self.center,
            radius,
        }
    }

    /// Determines the intersection of this OBB with a plane.
    pub fn intersect_plane(&self, normal: DVec3, distance: f64) -> Intersect {
        let u = self.half_axes.x_axis;
        let v = self.half_axes.y_axis;
        let w = self.half_axes.z_axis;

        let rad_effective =
            normal.dot(u).abs() + normal.dot(v).abs() + normal.dot(w).abs();
        let dist_to_center = normal.dot(self.center) + distance;

        if dist_to_center > rad_effective {
            Intersect::Outside
        } else if dist_to_center < -rad_effective {
            Intersect::Inside
        } else {
            Intersect::Intersecting
        }
    }
}

/// An axis-aligned bounding box defined by minimum and maximum corners.
/// Maps to CesiumJS `AxisAlignedBoundingBox`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AxisAlignedBoundingBox {
    /// The minimum corner.
    pub minimum: DVec3,
    /// The maximum corner.
    pub maximum: DVec3,
    /// The center (computed).
    pub center: DVec3,
}

impl AxisAlignedBoundingBox {
    pub fn new(minimum: DVec3, maximum: DVec3) -> Self {
        let center = (minimum + maximum) * 0.5;
        Self {
            minimum,
            maximum,
            center,
        }
    }

    /// Creates an AABB from a set of points.
    /// Maps to `AxisAlignedBoundingBox.fromPoints`
    pub fn from_points(points: &[DVec3]) -> Self {
        if points.is_empty() {
            return Self::new(DVec3::ZERO, DVec3::ZERO);
        }

        let mut minimum = DVec3::new(f64::MAX, f64::MAX, f64::MAX);
        let mut maximum = DVec3::new(f64::MIN, f64::MIN, f64::MIN);

        for p in points {
            minimum = minimum.min(*p);
            maximum = maximum.max(*p);
        }

        Self::new(minimum, maximum)
    }

    /// Determines if a point is inside the AABB.
    pub fn contains(&self, point: DVec3) -> bool {
        point.x >= self.minimum.x
            && point.x <= self.maximum.x
            && point.y >= self.minimum.y
            && point.y <= self.maximum.y
            && point.z >= self.minimum.z
            && point.z <= self.maximum.z
    }

    /// Computes the union of two AABBs.
    pub fn union(&self, other: &Self) -> Self {
        Self::new(
            self.minimum.min(other.minimum),
            self.maximum.max(other.maximum),
        )
    }

    /// Converts to a bounding sphere.
    pub fn to_bounding_sphere(&self) -> BoundingSphere {
        let center = self.center;
        let radius = (self.maximum - self.minimum).length() * 0.5;
        BoundingSphere { center, radius }
    }

    /// Determines the intersection with a plane.
    pub fn intersect_plane(&self, normal: DVec3, distance: f64) -> Intersect {
        let center_dist = normal.dot(self.center) + distance;
        let half_extents = (self.maximum - self.minimum) * 0.5;
        let rad_effective =
            normal.x.abs() * half_extents.x + normal.y.abs() * half_extents.y + normal.z.abs() * half_extents.z;

        if center_dist > rad_effective {
            Intersect::Outside
        } else if center_dist < -rad_effective {
            Intersect::Inside
        } else {
            Intersect::Intersecting
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_sphere_from_points() {
        let points = vec![
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(-1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec3::new(0.0, -1.0, 0.0),
        ];
        let bs = BoundingSphere::from_points(&points);
        assert!(bs.center.abs_diff_eq(DVec3::ZERO, 1e-10));
        assert!((bs.radius - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_bounding_sphere_contains() {
        let bs = BoundingSphere::new(DVec3::ZERO, 5.0);
        assert!(bs.contains(DVec3::new(3.0, 0.0, 0.0)));
        assert!(!bs.contains(DVec3::new(6.0, 0.0, 0.0)));
    }

    #[test]
    fn test_bounding_sphere_union() {
        let a = BoundingSphere::new(DVec3::ZERO, 1.0);
        let b = BoundingSphere::new(DVec3::new(3.0, 0.0, 0.0), 1.0);
        let u = a.union(&b);
        assert!((u.center.x - 1.5).abs() < 1e-10);
        assert!((u.radius - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_aabb_from_points() {
        let points = vec![
            DVec3::new(1.0, 2.0, 3.0),
            DVec3::new(-1.0, -2.0, -3.0),
            DVec3::new(0.5, 0.5, 0.5),
        ];
        let aabb = AxisAlignedBoundingBox::from_points(&points);
        assert_eq!(aabb.minimum, DVec3::new(-1.0, -2.0, -3.0));
        assert_eq!(aabb.maximum, DVec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_obb_distance_to() {
        let obb = OrientedBoundingBox::new(
            DVec3::ZERO,
            DMat3::from_cols(
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.0, 1.0, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
            ),
        );
        // Point outside along x
        let dist = obb.distance_to(DVec3::new(3.0, 0.0, 0.0));
        assert!((dist - 2.0).abs() < 1e-10);
        // Point inside
        let dist = obb.distance_to(DVec3::new(0.5, 0.0, 0.0));
        assert!((dist - 0.0).abs() < 1e-10);
    }
}
