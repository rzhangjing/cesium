//! Ray, Plane, and intersection tests.
//! Maps to CesiumJS `Core/Ray.js`, `Core/Plane.js`, `Core/IntersectionTests.js`, `Core/Intersections2D.js`

use crate::bounding::{AxisAlignedBoundingBox, BoundingSphere, OrientedBoundingBox};
use crate::ellipsoid::Ellipsoid;
use crate::math_utils::EPSILON15;
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// The result of an intersection test with a plane or culling volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intersect {
    /// The object is entirely outside.
    Outside,
    /// The object intersects the boundary.
    Intersecting,
    /// The object is entirely inside.
    Inside,
}

/// A ray defined by an origin and direction.
/// Maps to CesiumJS `Ray`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ray {
    /// The origin of the ray.
    pub origin: DVec3,
    /// The direction of the ray (normalized).
    pub direction: DVec3,
}

impl Ray {
    pub fn new(origin: DVec3, direction: DVec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// Gets a point along the ray at parameter t.
    #[inline]
    pub fn point_at(&self, t: f64) -> DVec3 {
        self.origin + self.direction * t
    }
}

/// A plane defined by a normal and distance from origin.
/// The plane equation is: normal · x + distance = 0
/// Maps to CesiumJS `Plane`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Plane {
    /// The plane normal (normalized).
    pub normal: DVec3,
    /// The shortest distance from the origin to the plane.
    pub distance: f64,
}

impl Plane {
    /// Creates a plane from a normal and a point on the plane.
    /// Maps to `Plane.fromPointNormal`
    pub fn from_point_normal(point: DVec3, normal: DVec3) -> Self {
        let normal = normal.normalize();
        let distance = -normal.dot(point);
        Self { normal, distance }
    }

    /// Creates a plane from a normal and distance.
    pub fn new(normal: DVec3, distance: f64) -> Self {
        Self {
            normal: normal.normalize(),
            distance,
        }
    }

    /// Computes the signed distance from a point to the plane.
    /// Maps to `Plane.getPointDistance`
    pub fn point_distance(&self, point: DVec3) -> f64 {
        self.normal.dot(point) + self.distance
    }

    /// Projects a point onto the plane.
    pub fn project_point_onto_plane(&self, point: DVec3) -> DVec3 {
        let dist = self.point_distance(point);
        point - self.normal * dist
    }
}

// --- Intersection Tests ---
// Maps to CesiumJS `IntersectionTests`

/// Computes the intersection of a ray with an ellipsoid.
/// Returns (t0, t1) parameters along the ray, or None if no intersection.
/// Maps to `IntersectionTests.rayEllipsoid`
pub fn ray_ellipsoid(ray: &Ray, ellipsoid: &Ellipsoid) -> Option<(f64, f64)> {
    ellipsoid.intersection(ray.origin, ray.direction)
}

/// Computes the intersection of a ray with a plane.
/// Returns the intersection point, or None if parallel.
/// Maps to `IntersectionTests.rayPlane`
pub fn ray_plane(ray: &Ray, plane: &Plane) -> Option<DVec3> {
    let denominator = plane.normal.dot(ray.direction);
    if denominator.abs() < EPSILON15 {
        return None;
    }
    let t = -(plane.normal.dot(ray.origin) + plane.distance) / denominator;
    if t < 0.0 {
        return None;
    }
    Some(ray.point_at(t))
}

/// Computes the intersection of a ray with a bounding sphere.
/// Returns the nearest intersection point, or None.
/// Maps to `IntersectionTests.raySphere`
pub fn ray_sphere(ray: &Ray, sphere: &BoundingSphere) -> Option<DVec3> {
    let oc = ray.origin - sphere.center;
    let a = ray.direction.dot(ray.direction);
    let b = 2.0 * oc.dot(ray.direction);
    let c = oc.dot(oc) - sphere.radius * sphere.radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return None;
    }

    let sqrt_disc = discriminant.sqrt();
    let t0 = (-b - sqrt_disc) / (2.0 * a);
    let t1 = (-b + sqrt_disc) / (2.0 * a);

    let t = if t0 >= 0.0 {
        t0
    } else if t1 >= 0.0 {
        t1
    } else {
        return None;
    };

    Some(ray.point_at(t))
}

/// Computes the intersection of a ray with a triangle (Möller–Trumbore algorithm).
/// Returns the intersection point, or None.
/// Maps to `IntersectionTests.rayTriangle`
pub fn ray_triangle(ray: &Ray, v0: DVec3, v1: DVec3, v2: DVec3) -> Option<DVec3> {
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let h = ray.direction.cross(edge2);
    let a = edge1.dot(h);

    if a.abs() < EPSILON15 {
        return None; // Ray parallel to triangle
    }

    let f = 1.0 / a;
    let s = ray.origin - v0;
    let u = f * s.dot(h);

    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = s.cross(edge1);
    let v = f * ray.direction.dot(q);

    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * edge2.dot(q);
    if t < EPSILON15 {
        return None;
    }

    Some(ray.point_at(t))
}

/// Computes the intersection of a ray with an oriented bounding box.
/// Returns the distance along the ray, or None.
/// Maps to `IntersectionTests.rayOrientedBoundingBox`
pub fn ray_obb(ray: &Ray, obb: &OrientedBoundingBox) -> Option<f64> {
    let offset = ray.origin - obb.center;

    let u = obb.half_axes.x_axis;
    let v = obb.half_axes.y_axis;
    let w = obb.half_axes.z_axis;

    // Transform ray into OBB local space
    let inv_u = if u.length_squared() > 0.0 { u / u.length_squared() } else { DVec3::ZERO };
    let inv_v = if v.length_squared() > 0.0 { v / v.length_squared() } else { DVec3::ZERO };
    let inv_w = if w.length_squared() > 0.0 { w / w.length_squared() } else { DVec3::ZERO };

    let origin_local = DVec3::new(
        offset.dot(inv_u),
        offset.dot(inv_v),
        offset.dot(inv_w),
    );
    let dir_local = DVec3::new(
        ray.direction.dot(inv_u),
        ray.direction.dot(inv_v),
        ray.direction.dot(inv_w),
    );

    // Slab method for unit box [-1, 1]^3
    let mut t_min = f64::NEG_INFINITY;
    let mut t_max = f64::INFINITY;

    for i in 0..3 {
        let o = [origin_local.x, origin_local.y, origin_local.z][i];
        let d = [dir_local.x, dir_local.y, dir_local.z][i];

        if d.abs() < EPSILON15 {
            if !(-1.0..=1.0).contains(&o) {
                return None;
            }
        } else {
            let inv_d = 1.0 / d;
            let mut t1 = (-1.0 - o) * inv_d;
            let mut t2 = (1.0 - o) * inv_d;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            t_min = t_min.max(t1);
            t_max = t_max.min(t2);
            if t_min > t_max {
                return None;
            }
        }
    }

    if t_max < 0.0 {
        return None;
    }

    Some(if t_min >= 0.0 { t_min } else { t_max })
}

/// Computes the intersection of a ray with an axis-aligned bounding box.
/// Returns the distance along the ray, or None.
/// Maps to `IntersectionTests.rayAxisAlignedBoundingBox`
pub fn ray_aabb(ray: &Ray, aabb: &AxisAlignedBoundingBox) -> Option<f64> {
    let mut t_min = f64::NEG_INFINITY;
    let mut t_max = f64::INFINITY;

    for i in 0..3 {
        let o = [ray.origin.x, ray.origin.y, ray.origin.z][i];
        let d = [ray.direction.x, ray.direction.y, ray.direction.z][i];
        let min = [aabb.minimum.x, aabb.minimum.y, aabb.minimum.z][i];
        let max = [aabb.maximum.x, aabb.maximum.y, aabb.maximum.z][i];

        if d.abs() < EPSILON15 {
            if o < min || o > max {
                return None;
            }
        } else {
            let inv_d = 1.0 / d;
            let mut t1 = (min - o) * inv_d;
            let mut t2 = (max - o) * inv_d;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            t_min = t_min.max(t1);
            t_max = t_max.min(t2);
            if t_min > t_max {
                return None;
            }
        }
    }

    if t_max < 0.0 {
        return None;
    }

    Some(if t_min >= 0.0 { t_min } else { t_max })
}

// --- 2D Intersection Tests ---
// Maps to CesiumJS `Intersections2D`

/// Computes the barycentric coordinates of a point in a triangle.
/// Maps to `Intersections2D.computeBarycentricCoordinates`
#[allow(clippy::too_many_arguments)]
pub fn compute_barycentric_coordinates(
    point_x: f64,
    point_y: f64,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x3: f64,
    y3: f64,
) -> (f64, f64, f64) {
    let x1mx3 = x1 - x3;
    let x3mx2 = x3 - x2;
    let y2my3 = y2 - y3;
    let y1my3 = y1 - y3;
    let inverse_det = 1.0 / (y2my3 * x1mx3 + x3mx2 * y1my3);
    let dpx = point_x - x3;
    let dpy = point_y - y3;

    let u = (y2my3 * dpx + x3mx2 * dpy) * inverse_det;
    let v = (-y1my3 * dpx + x1mx3 * dpy) * inverse_det;
    let w = 1.0 - u - v;
    (u, v, w)
}

/// Clips a triangle against an axis-aligned clip rectangle.
/// Maps to `Intersections2D.clipTriangleAtAxisAlignedThreshold`
#[allow(clippy::too_many_arguments)]
pub fn clip_triangle_at_axis_aligned_threshold(
    threshold: f64,
    keep_above: bool,
    u0: f64,
    v0: f64,
    u1: f64,
    v1: f64,
    u2: f64,
    v2: f64,
) -> Vec<(f64, f64)> {
    // Simplified: returns vertices that satisfy the condition
    let mut result = Vec::new();
    let points = [(u0, v0), (u1, v1), (u2, v2)];

    for &(u, v) in &points {
        let val = if keep_above { u } else { -u };
        if val >= threshold {
            result.push((u, v));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_plane_intersection() {
        let ray = Ray::new(DVec3::new(0.0, 0.0, 5.0), DVec3::new(0.0, 0.0, -1.0));
        let plane = Plane::from_point_normal(DVec3::ZERO, DVec3::new(0.0, 0.0, 1.0));
        let hit = ray_plane(&ray, &plane).unwrap();
        assert!(hit.abs_diff_eq(DVec3::ZERO, 1e-10));
    }

    #[test]
    fn test_ray_plane_parallel() {
        let ray = Ray::new(DVec3::new(0.0, 0.0, 5.0), DVec3::new(1.0, 0.0, 0.0));
        let plane = Plane::from_point_normal(DVec3::ZERO, DVec3::new(0.0, 0.0, 1.0));
        assert!(ray_plane(&ray, &plane).is_none());
    }

    #[test]
    fn test_ray_sphere_hit() {
        let ray = Ray::new(DVec3::new(0.0, 0.0, 5.0), DVec3::new(0.0, 0.0, -1.0));
        let sphere = BoundingSphere::new(DVec3::ZERO, 1.0);
        let hit = ray_sphere(&ray, &sphere).unwrap();
        assert!((hit.z - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ray_sphere_miss() {
        let ray = Ray::new(DVec3::new(0.0, 5.0, 5.0), DVec3::new(0.0, 0.0, -1.0));
        let sphere = BoundingSphere::new(DVec3::ZERO, 1.0);
        assert!(ray_sphere(&ray, &sphere).is_none());
    }

    #[test]
    fn test_ray_triangle_hit() {
        let ray = Ray::new(DVec3::new(0.25, 0.25, 1.0), DVec3::new(0.0, 0.0, -1.0));
        let v0 = DVec3::new(0.0, 0.0, 0.0);
        let v1 = DVec3::new(1.0, 0.0, 0.0);
        let v2 = DVec3::new(0.0, 1.0, 0.0);
        let hit = ray_triangle(&ray, v0, v1, v2).unwrap();
        assert!((hit.z).abs() < 1e-10);
    }

    #[test]
    fn test_ray_triangle_miss() {
        let ray = Ray::new(DVec3::new(2.0, 2.0, 1.0), DVec3::new(0.0, 0.0, -1.0));
        let v0 = DVec3::new(0.0, 0.0, 0.0);
        let v1 = DVec3::new(1.0, 0.0, 0.0);
        let v2 = DVec3::new(0.0, 1.0, 0.0);
        assert!(ray_triangle(&ray, v0, v1, v2).is_none());
    }

    #[test]
    fn test_ray_aabb_hit() {
        let ray = Ray::new(DVec3::new(0.0, 0.0, 5.0), DVec3::new(0.0, 0.0, -1.0));
        let aabb = AxisAlignedBoundingBox::new(DVec3::new(-1.0, -1.0, -1.0), DVec3::new(1.0, 1.0, 1.0));
        let t = ray_aabb(&ray, &aabb).unwrap();
        assert!((t - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_barycentric_coordinates() {
        let (u, v, w) = compute_barycentric_coordinates(0.25, 0.25, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0);
        assert!((u - 0.5).abs() < 1e-10);
        assert!((v - 0.25).abs() < 1e-10);
        assert!((w - 0.25).abs() < 1e-10);
    }
}
