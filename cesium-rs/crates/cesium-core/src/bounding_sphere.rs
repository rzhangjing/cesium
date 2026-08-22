//! Ported from `packages/engine/Source/Core/BoundingSphere.js`.
//!
//! A bounding sphere with a center and a radius.

use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;
use crate::intersect::Intersect;
use crate::math::CesiumMath;
use crate::matrix4::Matrix4;
use crate::plane::Plane;

/// A bounding sphere with a center and a radius.
#[derive(Debug, Clone)]
pub struct BoundingSphere {
    /// The center point of the sphere.
    pub center: Cartesian3,
    /// The radius of the sphere.
    pub radius: f64,
}

impl Default for BoundingSphere {
    fn default() -> Self {
        Self {
            center: Cartesian3::ZERO,
            radius: 0.0,
        }
    }
}

impl PartialEq for BoundingSphere {
    fn eq(&self, other: &Self) -> bool {
        self.center == other.center && self.radius == other.radius
    }
}

const VOLUME_CONSTANT: f64 = (4.0 / 3.0) * CesiumMath::PI;

/// Helper: compute magnitude of the difference between two points.
fn distance_between(a: &Cartesian3, b: &Cartesian3) -> f64 {
    Cartesian3::magnitude(&Cartesian3::subtract_new(a, b))
}

/// Helper: compute squared magnitude of the difference between two points.
fn distance_squared_between(a: &Cartesian3, b: &Cartesian3) -> f64 {
    Cartesian3::magnitude_squared(&Cartesian3::subtract_new(a, b))
}

/// Internal helper: compute the Ritter/naive bounding sphere from min/max
/// corner tracking data and a list of positions accessed via a closure.
fn compute_bounding_sphere<F>(
    positions_count: usize,
    get_pos: F,
) -> (Cartesian3, f64)
where
    F: Fn(usize) -> Cartesian3,
{
    if positions_count == 0 {
        return (Cartesian3::ZERO, 0.0);
    }

    let mut current_pos = get_pos(0);
    let mut x_min = current_pos;
    let mut y_min = current_pos;
    let mut z_min = current_pos;
    let mut x_max = current_pos;
    let mut y_max = current_pos;
    let mut z_max = current_pos;

    for i in 1..positions_count {
        current_pos = get_pos(i);
        if current_pos.x < x_min.x { x_min = current_pos; }
        if current_pos.x > x_max.x { x_max = current_pos; }
        if current_pos.y < y_min.y { y_min = current_pos; }
        if current_pos.y > y_max.y { y_max = current_pos; }
        if current_pos.z < z_min.z { z_min = current_pos; }
        if current_pos.z > z_max.z { z_max = current_pos; }
    }

    let x_span = distance_between(&x_max, &x_min);
    let y_span = distance_between(&y_max, &y_min);
    let z_span = distance_between(&z_max, &z_min);

    let (diameter1, diameter2);
    if x_span * x_span >= y_span * y_span && x_span * x_span >= z_span * z_span {
        diameter1 = x_min; diameter2 = x_max;
    } else if y_span * y_span >= z_span * z_span {
        diameter1 = y_min; diameter2 = y_max;
    } else {
        diameter1 = z_min; diameter2 = z_max;
    }

    let mut ritter_center = Cartesian3::new(
        (diameter1.x + diameter2.x) * 0.5,
        (diameter1.y + diameter2.y) * 0.5,
        (diameter1.z + diameter2.z) * 0.5,
    );

    let mut radius_squared = distance_squared_between(&diameter2, &ritter_center);
    let mut ritter_radius = radius_squared.sqrt();

    let min_box_pt = Cartesian3::new(x_min.x, y_min.y, z_min.z);
    let max_box_pt = Cartesian3::new(x_max.x, y_max.y, z_max.z);
    let naive_center = Cartesian3::midpoint_new(&min_box_pt, &max_box_pt);

    let mut naive_radius = 0.0;
    for i in 0..positions_count {
        let pos = get_pos(i);
        let dist = distance_between(&pos, &naive_center);
        if dist > naive_radius { naive_radius = dist; }

        let old_center_to_point_squared = distance_squared_between(&pos, &ritter_center);
        if old_center_to_point_squared > radius_squared {
            let old_center_to_point = old_center_to_point_squared.sqrt();
            ritter_radius = (ritter_radius + old_center_to_point) * 0.5;
            radius_squared = ritter_radius * ritter_radius;
            let old_to_new = old_center_to_point - ritter_radius;
            ritter_center.x =
                (ritter_radius * ritter_center.x + old_to_new * pos.x) / old_center_to_point;
            ritter_center.y =
                (ritter_radius * ritter_center.y + old_to_new * pos.y) / old_center_to_point;
            ritter_center.z =
                (ritter_radius * ritter_center.z + old_to_new * pos.z) / old_center_to_point;
        }
    }

    if ritter_radius < naive_radius {
        (ritter_center, ritter_radius)
    } else {
        (naive_center, naive_radius)
    }
}

impl BoundingSphere {
    /// Creates a new `BoundingSphere`.
    pub fn new(center: Cartesian3, radius: f64) -> Self {
        Self { center, radius }
    }

    /// Computes a tight-fitting bounding sphere enclosing a list of 3D Cartesian points.
    pub fn from_points(positions: &[Cartesian3], result: Option<Self>) -> Self {
        let mut r = result.unwrap_or_default();
        if positions.is_empty() {
            r.center = Cartesian3::ZERO;
            r.radius = 0.0;
            return r;
        }
        let (center, radius) = compute_bounding_sphere(positions.len(), |i| positions[i]);
        r.center = center;
        r.radius = radius;
        r
    }

    /// Computes a bounding sphere from flat X,Y,Z positions with optional center offset and stride.
    pub fn from_vertices(
        positions: &[f64],
        center: Option<&Cartesian3>,
        stride: Option<usize>,
        result: Option<Self>,
    ) -> Self {
        let mut r = result.unwrap_or_default();
        if positions.is_empty() {
            r.center = Cartesian3::ZERO;
            r.radius = 0.0;
            return r;
        }

        let center_offset = center.cloned().unwrap_or(Cartesian3::ZERO);
        let stride = stride.unwrap_or(3);
        debug_assert!(stride >= 3, "stride must be at least 3");

        let num_vertices = positions.len() / stride;
        let (ctr, rad) = compute_bounding_sphere(num_vertices, |i| {
            let base = i * stride;
            Cartesian3::new(
                positions[base] + center_offset.x,
                positions[base + 1] + center_offset.y,
                positions[base + 2] + center_offset.z,
            )
        });
        r.center = ctr;
        r.radius = rad;
        r
    }

    /// Computes a bounding sphere from the corner points of an axis-aligned bounding box.
    pub fn from_corner_points(
        corner: &Cartesian3,
        opposite_corner: &Cartesian3,
        result: Option<Self>,
    ) -> Self {
        let mut r = result.unwrap_or_default();
        r.center = Cartesian3::midpoint_new(corner, opposite_corner);
        r.radius = Cartesian3::distance(&r.center, opposite_corner);
        r
    }

    /// Creates a bounding sphere encompassing an ellipsoid.
    pub fn from_ellipsoid(ellipsoid: &Ellipsoid, result: Option<Self>) -> Self {
        let mut r = result.unwrap_or_default();
        r.center = Cartesian3::ZERO;
        r.radius = ellipsoid.maximum_radius();
        r
    }

    /// Computes a bounding sphere that contains both bounding spheres.
    pub fn union(left: &Self, right: &Self, result: Option<Self>) -> Self {
        let mut r = result.unwrap_or_default();

        let to_right_center = Cartesian3::subtract_new(&right.center, &left.center);
        let center_separation = Cartesian3::magnitude(&to_right_center);

        if left.radius >= center_separation + right.radius {
            return left.clone();
        }
        if right.radius >= center_separation + left.radius {
            return right.clone();
        }

        let half_distance = (left.radius + center_separation + right.radius) * 0.5;
        let scale = (-left.radius + half_distance) / center_separation;
        let center = Cartesian3::add_new(
            &Cartesian3::multiply_by_scalar_new(&to_right_center, scale),
            &left.center,
        );
        r.center = center;
        r.radius = half_distance;
        r
    }

    /// Computes a bounding sphere by enlarging the provided sphere to contain the provided point.
    pub fn expand(sphere: &Self, point: &Cartesian3, _result: Option<Self>) -> Self {
        let mut r = sphere.clone();
        let diff = Cartesian3::subtract_new(point, &r.center);
        let dist = Cartesian3::magnitude(&diff);
        if dist > r.radius {
            r.radius = dist;
        }
        r
    }

    /// Determines which side of a plane a sphere is located.
    pub fn intersect_plane(sphere: &Self, plane: &Plane) -> Intersect {
        let distance_to_plane =
            Cartesian3::dot(&plane.normal, &sphere.center) + plane.distance;
        let radius = sphere.radius;
        if distance_to_plane < -radius {
            Intersect::Outside
        } else if distance_to_plane < radius {
            Intersect::Intersecting
        } else {
            Intersect::Inside
        }
    }

    /// Applies a 4x4 affine transformation matrix to a bounding sphere.
    pub fn transform(sphere: &Self, transform: &Matrix4, result: Option<Self>) -> Self {
        let mut r = result.unwrap_or_default();
        r.center = Matrix4::multiply_by_point_new(transform, &sphere.center);
        r.radius = Matrix4::get_maximum_scale(transform) * sphere.radius;
        r
    }

    /// Computes the estimated distance squared from the closest point on a bounding sphere to a point.
    pub fn distance_squared_to(sphere: &Self, cartesian: &Cartesian3) -> f64 {
        let diff = Cartesian3::subtract_new(&sphere.center, cartesian);
        let distance = Cartesian3::magnitude(&diff) - sphere.radius;
        if distance <= 0.0 { 0.0 } else { distance * distance }
    }

    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize = 4;

    /// Stores the provided instance into the provided array.
    pub fn pack(&self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = self.center.x;
        array[starting_index + 1] = self.center.y;
        array[starting_index + 2] = self.center.z;
        array[starting_index + 3] = self.radius;
    }

    /// Retrieves an instance from a packed array.
    pub fn unpack(array: &[f64], starting_index: usize, result: Option<Self>) -> Self {
        let mut r = result.unwrap_or_default();
        r.center.x = array[starting_index];
        r.center.y = array[starting_index + 1];
        r.center.z = array[starting_index + 2];
        r.radius = array[starting_index + 3];
        r
    }

    /// Computes the volume of the bounding sphere.
    pub fn volume(&self) -> f64 {
        VOLUME_CONSTANT * self.radius * self.radius * self.radius
    }

    /// Duplicates a BoundingSphere instance.
    pub fn clone_sphere(sphere: &Self, result: Option<Self>) -> Self {
        result.map_or_else(|| Self::new(sphere.center, sphere.radius), |mut r| {
            r.center = sphere.center;
            r.radius = sphere.radius;
            r
        })
    }

    /// Compares two bounding spheres componentwise.
    pub fn equals(left: &Self, right: &Self) -> bool {
        left.center == right.center && left.radius == right.radius
    }
}
