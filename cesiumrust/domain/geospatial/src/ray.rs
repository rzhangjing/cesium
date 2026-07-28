//! Ray, Plane, and intersection tests.
//! Maps to CesiumJS `Core/Ray.js`, `Core/Plane.js`, `Core/IntersectionTests.js`, `Core/Intersections2D.js`

use crate::bounding::{AxisAlignedBoundingBox, BoundingSphere, OrientedBoundingBox};
use crate::ellipsoid::Ellipsoid;
use crate::math_utils::{EPSILON15, EPSILON6};
use glam::{DMat4, DVec3, DVec4};
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
    /// The XY plane through the origin, normal +Z.
    /// Maps to `Plane.ORIGIN_XY_PLANE`
    pub const ORIGIN_XY_PLANE: Self = Self {
        normal: DVec3::Z,
        distance: 0.0,
    };
    /// The YZ plane through the origin, normal +X.
    /// Maps to `Plane.ORIGIN_YZ_PLANE`
    pub const ORIGIN_YZ_PLANE: Self = Self {
        normal: DVec3::X,
        distance: 0.0,
    };
    /// The ZX plane through the origin, normal +Y.
    /// Maps to `Plane.ORIGIN_ZX_PLANE`
    pub const ORIGIN_ZX_PLANE: Self = Self {
        normal: DVec3::Y,
        distance: 0.0,
    };

    /// Creates a plane from a normal and distance.
    ///
    /// Faithful to CesiumJS: the normal is stored **as-is** (not re-normalized);
    /// the caller must supply a unit-length normal. The debug-only normalization
    /// check mirrors CesiumJS's `DeveloperError` (stripped in release builds).
    /// Maps to `new Plane(normal, distance)`
    pub fn new(normal: DVec3, distance: f64) -> Self {
        debug_assert!(
            (normal.length() - 1.0).abs() <= crate::math_utils::EPSILON6,
            "normal must be normalized"
        );
        Self { normal, distance }
    }

    /// Creates a plane from a point and a (unit-length) normal.
    /// Maps to `Plane.fromPointNormal`
    pub fn from_point_normal(point: DVec3, normal: DVec3) -> Self {
        debug_assert!(
            (normal.length() - 1.0).abs() <= crate::math_utils::EPSILON6,
            "normal must be normalized"
        );
        let distance = -normal.dot(point);
        Self { normal, distance }
    }

    /// Creates a plane from the general equation coefficients `(x, y, z, w)`,
    /// where `(x, y, z)` is the unit-length normal and `w` is the distance.
    /// Maps to `Plane.fromCartesian4`
    pub fn from_cartesian4(coefficients: DVec4) -> Self {
        let normal = coefficients.truncate();
        debug_assert!(
            (normal.length() - 1.0).abs() <= crate::math_utils::EPSILON6,
            "normal must be normalized"
        );
        Self {
            normal,
            distance: coefficients.w,
        }
    }

    /// Computes the signed distance from a point to the plane.
    /// Maps to `Plane.getPointDistance`
    pub fn point_distance(&self, point: DVec3) -> f64 {
        self.normal.dot(point) + self.distance
    }

    /// Projects a point onto the plane.
    /// Maps to `Plane.projectPointOntoPlane`
    pub fn project_point_onto_plane(&self, point: DVec3) -> DVec3 {
        let dist = self.point_distance(point);
        point - self.normal * dist
    }

    /// Transforms the plane by the given transformation matrix.
    ///
    /// Faithful port: multiplies the plane-as-Cartesian4 by the inverse-transpose
    /// of the transform, then renormalizes to Hessian Normal Form.
    /// Maps to `Plane.transform`
    pub fn transform(&self, transform: &DMat4) -> Self {
        let inverse_transpose = transform.inverse().transpose();
        let mut plane_as_cartesian4 =
            DVec4::new(self.normal.x, self.normal.y, self.normal.z, self.distance);
        plane_as_cartesian4 = inverse_transpose * plane_as_cartesian4;
        let transformed_normal = plane_as_cartesian4.truncate();
        plane_as_cartesian4 /= transformed_normal.length();
        Plane::from_cartesian4(plane_as_cartesian4)
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

/// Computes the intersection of a line segment with a plane.
/// Returns the intersection point, or None if the segment doesn't cross the plane.
/// Maps to `IntersectionTests.lineSegmentPlane`
pub fn line_segment_plane(p0: DVec3, p1: DVec3, plane: &Plane) -> Option<DVec3> {
    let difference = p1 - p0;
    let n = plane.normal.dot(difference);
    if n.abs() < EPSILON6 {
        return None;
    }
    let t = -(plane.distance + plane.normal.dot(p0)) / n;
    if t < 0.0 || t > 1.0 {
        return None;
    }
    Some(p0 + difference * t)
}

/// Computes the intersection of a ray with a bounding sphere.
/// Returns an interval (start, stop) of parametric distances along the ray,
/// or None if there is no intersection.
/// Maps to `IntersectionTests.raySphere`
pub fn ray_sphere(ray: &Ray, sphere: &BoundingSphere) -> Option<(f64, f64)> {
    let origin = ray.origin;
    let direction = ray.direction;
    let center = sphere.center;
    let radius_squared = sphere.radius * sphere.radius;

    let diff = origin - center;

    let a = direction.dot(direction);
    let b = 2.0 * direction.dot(diff);
    let c = diff.dot(diff) - radius_squared;

    let det = b * b - 4.0 * a * c;
    if det < 0.0 {
        return None;
    }

    let (root0, root1) = if det > 0.0 {
        let denom = 1.0 / (2.0 * a);
        let disc = det.sqrt();
        let r0 = (-b + disc) * denom;
        let r1 = (-b - disc) * denom;
        if r0 < r1 { (r0, r1) } else { (r1, r0) }
    } else {
        // det == 0: repeated root
        let root = -b / (2.0 * a);
        if root == 0.0 {
            return None;
        }
        (root, root)
    };

    // Public API: filter and clamp
    if root1 < 0.0 {
        return None;
    }
    let start = root0.max(0.0);
    Some((start, root1))
}

/// Computes the intersection of a ray with a triangle as a parametric distance.
/// Returns the parametric distance `t` along the ray, or None.
/// The result can be negative when the triangle is behind the ray.
/// Maps to `IntersectionTests.rayTriangleParametric`
pub fn ray_triangle_parametric(
    ray: &Ray,
    p0: DVec3,
    p1: DVec3,
    p2: DVec3,
    cull_back_faces: bool,
) -> Option<f64> {
    let origin = ray.origin;
    let direction = ray.direction;

    let edge0 = p1 - p0;
    let edge1 = p2 - p0;

    let p = direction.cross(edge1);
    let det = edge0.dot(p);

    if cull_back_faces {
        if det < EPSILON6 {
            return None;
        }

        let tvec = origin - p0;
        let u = tvec.dot(p);
        if u < 0.0 || u > det {
            return None;
        }

        let q = tvec.cross(edge0);
        let v = direction.dot(q);
        if v < 0.0 || u + v > det {
            return None;
        }

        Some(edge1.dot(q) / det)
    } else {
        if det.abs() < EPSILON6 {
            return None;
        }
        let inv_det = 1.0 / det;

        let tvec = origin - p0;
        let u = tvec.dot(p) * inv_det;
        if u < 0.0 || u > 1.0 {
            return None;
        }

        let q = tvec.cross(edge0);
        let v = direction.dot(q) * inv_det;
        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        Some(edge1.dot(q) * inv_det)
    }
}

/// Computes the intersection of a ray with a triangle (Möller–Trumbore algorithm).
/// Returns the intersection point, or None.
/// Maps to `IntersectionTests.rayTriangle`
pub fn ray_triangle(
    ray: &Ray,
    v0: DVec3,
    v1: DVec3,
    v2: DVec3,
    cull_back_faces: bool,
) -> Option<DVec3> {
    let t = ray_triangle_parametric(ray, v0, v1, v2, cull_back_faces)?;
    if t < 0.0 {
        return None;
    }
    Some(ray.point_at(t))
}

/// Computes the intersection of a line segment with a triangle.
/// Returns the intersection point, or None.
/// Maps to `IntersectionTests.lineSegmentTriangle`
pub fn line_segment_triangle(
    v0: DVec3,
    v1: DVec3,
    p0: DVec3,
    p1: DVec3,
    p2: DVec3,
    cull_back_faces: bool,
) -> Option<DVec3> {
    let direction = (v1 - v0).normalize();
    let ray = Ray { origin: v0, direction };

    let t = ray_triangle_parametric(&ray, p0, p1, p2, cull_back_faces)?;
    let segment_length = (v1 - v0).length();
    if t < 0.0 || t > segment_length {
        return None;
    }
    Some(ray.point_at(t))
}

/// Result of a triangle-plane intersection.
/// Contains the positions and indices of the resulting triangles.
#[derive(Debug, Clone)]
pub struct TrianglePlaneIntersectionResult {
    pub positions: Vec<DVec3>,
    pub indices: Vec<u32>,
}

/// Computes the intersection of a triangle and a plane.
/// Returns positions and indices of resulting sub-triangles, or None if no intersection.
/// Maps to `IntersectionTests.trianglePlaneIntersection`
pub fn triangle_plane_intersection(
    p0: DVec3,
    p1: DVec3,
    p2: DVec3,
    plane: &Plane,
) -> Option<TrianglePlaneIntersectionResult> {
    let plane_normal = plane.normal;
    let plane_d = plane.distance;
    let p0_behind = plane_normal.dot(p0) + plane_d < 0.0;
    let p1_behind = plane_normal.dot(p1) + plane_d < 0.0;
    let p2_behind = plane_normal.dot(p2) + plane_d < 0.0;

    let mut num_behind = 0u32;
    if p0_behind { num_behind += 1; }
    if p1_behind { num_behind += 1; }
    if p2_behind { num_behind += 1; }

    match num_behind {
        1 => {
            if p0_behind {
                let u1 = line_segment_plane(p0, p1, plane)?;
                let u2 = line_segment_plane(p0, p2, plane)?;
                Some(TrianglePlaneIntersectionResult {
                    positions: vec![p0, p1, p2, u1, u2],
                    indices: vec![0, 3, 4, 1, 2, 4, 1, 4, 3],
                })
            } else if p1_behind {
                let u1 = line_segment_plane(p1, p2, plane)?;
                let u2 = line_segment_plane(p1, p0, plane)?;
                Some(TrianglePlaneIntersectionResult {
                    positions: vec![p0, p1, p2, u1, u2],
                    indices: vec![1, 3, 4, 2, 0, 4, 2, 4, 3],
                })
            } else {
                // p2_behind
                let u1 = line_segment_plane(p2, p0, plane)?;
                let u2 = line_segment_plane(p2, p1, plane)?;
                Some(TrianglePlaneIntersectionResult {
                    positions: vec![p0, p1, p2, u1, u2],
                    indices: vec![2, 3, 4, 0, 1, 4, 0, 4, 3],
                })
            }
        }
        2 => {
            if !p0_behind {
                let u1 = line_segment_plane(p1, p0, plane)?;
                let u2 = line_segment_plane(p2, p0, plane)?;
                Some(TrianglePlaneIntersectionResult {
                    positions: vec![p0, p1, p2, u1, u2],
                    indices: vec![1, 2, 4, 1, 4, 3, 0, 3, 4],
                })
            } else if !p1_behind {
                let u1 = line_segment_plane(p2, p1, plane)?;
                let u2 = line_segment_plane(p0, p1, plane)?;
                Some(TrianglePlaneIntersectionResult {
                    positions: vec![p0, p1, p2, u1, u2],
                    indices: vec![2, 0, 4, 2, 4, 3, 1, 3, 4],
                })
            } else {
                // !p2_behind
                let u1 = line_segment_plane(p0, p2, plane)?;
                let u2 = line_segment_plane(p1, p2, plane)?;
                Some(TrianglePlaneIntersectionResult {
                    positions: vec![p0, p1, p2, u1, u2],
                    indices: vec![0, 1, 4, 0, 4, 3, 2, 3, 4],
                })
            }
        }
        // numBehind == 0 (all in front) or 3 (all behind): no intersection
        _ => None,
    }
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

/// Splits a 2D triangle at an axis-aligned threshold and returns the resulting polygon.
/// Returns a flat Vec<f64> where:
/// - Values 0, 1, 2 are original vertex indices
/// - Value -1 indicates a new interpolated vertex, followed by (from_idx, to_idx, ratio)
/// Maps to `Intersections2D.clipTriangleAtAxisAlignedThreshold`
pub fn clip_triangle_at_axis_aligned_threshold(
    threshold: f64,
    keep_above: bool,
    u0: f64,
    u1: f64,
    u2: f64,
) -> Vec<f64> {
    let mut result: Vec<f64> = Vec::new();

    let u0_behind: bool;
    let u1_behind: bool;
    let u2_behind: bool;
    if keep_above {
        u0_behind = u0 < threshold;
        u1_behind = u1 < threshold;
        u2_behind = u2 < threshold;
    } else {
        u0_behind = u0 > threshold;
        u1_behind = u1 > threshold;
        u2_behind = u2 > threshold;
    }

    let num_behind = (u0_behind as u8) + (u1_behind as u8) + (u2_behind as u8);

    if num_behind == 1 {
        if u0_behind {
            let u01_ratio = (threshold - u0) / (u1 - u0);
            let u02_ratio = (threshold - u0) / (u2 - u0);
            result.push(1.0);
            result.push(2.0);
            if u02_ratio != 1.0 {
                result.extend_from_slice(&[-1.0, 0.0, 2.0, u02_ratio]);
            }
            if u01_ratio != 1.0 {
                result.extend_from_slice(&[-1.0, 0.0, 1.0, u01_ratio]);
            }
        } else if u1_behind {
            let u12_ratio = (threshold - u1) / (u2 - u1);
            let u10_ratio = (threshold - u1) / (u0 - u1);
            result.push(2.0);
            result.push(0.0);
            if u10_ratio != 1.0 {
                result.extend_from_slice(&[-1.0, 1.0, 0.0, u10_ratio]);
            }
            if u12_ratio != 1.0 {
                result.extend_from_slice(&[-1.0, 1.0, 2.0, u12_ratio]);
            }
        } else if u2_behind {
            let u20_ratio = (threshold - u2) / (u0 - u2);
            let u21_ratio = (threshold - u2) / (u1 - u2);
            result.push(0.0);
            result.push(1.0);
            if u21_ratio != 1.0 {
                result.extend_from_slice(&[-1.0, 2.0, 1.0, u21_ratio]);
            }
            if u20_ratio != 1.0 {
                result.extend_from_slice(&[-1.0, 2.0, 0.0, u20_ratio]);
            }
        }
    } else if num_behind == 2 {
        if !u0_behind && u0 != threshold {
            let u10_ratio = (threshold - u1) / (u0 - u1);
            let u20_ratio = (threshold - u2) / (u0 - u2);
            result.push(0.0);
            result.extend_from_slice(&[-1.0, 1.0, 0.0, u10_ratio]);
            result.extend_from_slice(&[-1.0, 2.0, 0.0, u20_ratio]);
        } else if !u1_behind && u1 != threshold {
            let u21_ratio = (threshold - u2) / (u1 - u2);
            let u01_ratio = (threshold - u0) / (u1 - u0);
            result.push(1.0);
            result.extend_from_slice(&[-1.0, 2.0, 1.0, u21_ratio]);
            result.extend_from_slice(&[-1.0, 0.0, 1.0, u01_ratio]);
        } else if !u2_behind && u2 != threshold {
            let u02_ratio = (threshold - u0) / (u2 - u0);
            let u12_ratio = (threshold - u1) / (u2 - u1);
            result.push(2.0);
            result.extend_from_slice(&[-1.0, 0.0, 2.0, u02_ratio]);
            result.extend_from_slice(&[-1.0, 1.0, 2.0, u12_ratio]);
        }
    } else if num_behind != 3 {
        // Completely in front of threshold
        result.extend_from_slice(&[0.0, 1.0, 2.0]);
    }
    // else: completely behind → empty

    result
}

/// Computes the intersection of two 2D line segments.
/// Returns Some((x, y)) if they intersect, None if parallel/coincident/non-intersecting.
/// Maps to `Intersections2D.computeLineSegmentLineSegmentIntersection`
#[allow(clippy::too_many_arguments)]
pub fn compute_line_segment_line_segment_intersection(
    x00: f64,
    y00: f64,
    x01: f64,
    y01: f64,
    x10: f64,
    y10: f64,
    x11: f64,
    y11: f64,
) -> Option<(f64, f64)> {
    let numerator1_a = (x11 - x10) * (y00 - y10) - (y11 - y10) * (x00 - x10);
    let numerator1_b = (x01 - x00) * (y00 - y10) - (y01 - y00) * (x00 - x10);
    let denominator1 = (y11 - y10) * (x01 - x00) - (x11 - x10) * (y01 - y00);

    if denominator1 == 0.0 {
        return None;
    }

    let ua1 = numerator1_a / denominator1;
    let ub1 = numerator1_b / denominator1;

    if ua1 >= 0.0 && ua1 <= 1.0 && ub1 >= 0.0 && ub1 <= 1.0 {
        let x = x00 + ua1 * (x01 - x00);
        let y = y00 + ua1 * (y01 - y00);
        return Some((x, y));
    }

    None
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
        let (start, stop) = ray_sphere(&ray, &sphere).unwrap();
        assert!((start - 4.0).abs() < 1e-10);
        assert!((stop - 6.0).abs() < 1e-10);
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
        let hit = ray_triangle(&ray, v0, v1, v2, false).unwrap();
        assert!((hit.z).abs() < 1e-10);
    }

    #[test]
    fn test_ray_triangle_miss() {
        let ray = Ray::new(DVec3::new(2.0, 2.0, 1.0), DVec3::new(0.0, 0.0, -1.0));
        let v0 = DVec3::new(0.0, 0.0, 0.0);
        let v1 = DVec3::new(1.0, 0.0, 0.0);
        let v2 = DVec3::new(0.0, 1.0, 0.0);
        assert!(ray_triangle(&ray, v0, v1, v2, false).is_none());
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
