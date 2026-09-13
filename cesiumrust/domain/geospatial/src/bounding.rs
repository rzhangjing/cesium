//! Bounding volumes - BoundingSphere, OrientedBoundingBox, AxisAlignedBoundingBox.
//! Maps to CesiumJS `Core/BoundingSphere.js`, `Core/OrientedBoundingBox.js`, `Core/AxisAlignedBoundingBox.js`

use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::math_utils::{self, EPSILON10, EPSILON15, EPSILON20, PI_F64, TWO_PI};
use crate::projection::MapProjection;
use crate::ray::{ray_plane, Intersect, Plane, Ray};
use crate::rectangle::Rectangle;
use glam::{DMat3, DMat4, DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// A bounding sphere defined by a center point and radius.
/// Maps to CesiumJS `BoundingSphere`
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
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

    /// Computes a tight-fitting bounding sphere enclosing a list of 3D points.
    /// Runs both a naive algorithm and Ritter's algorithm and returns the smaller sphere.
    /// Maps to `BoundingSphere.fromPoints`
    pub fn from_points(points: &[DVec3]) -> Self {
        if points.is_empty() {
            return Self {
                center: DVec3::ZERO,
                radius: 0.0,
            };
        }

        // Find the points with the smallest/largest x, y, z components.
        let mut x_min = points[0];
        let mut y_min = points[0];
        let mut z_min = points[0];
        let mut x_max = points[0];
        let mut y_max = points[0];
        let mut z_max = points[0];
        for &p in points.iter().skip(1) {
            if p.x < x_min.x {
                x_min = p;
            }
            if p.x > x_max.x {
                x_max = p;
            }
            if p.y < y_min.y {
                y_min = p;
            }
            if p.y > y_max.y {
                y_max = p;
            }
            if p.z < z_min.z {
                z_min = p;
            }
            if p.z > z_max.z {
                z_max = p;
            }
        }

        Self::from_points_with_extremes(points, x_min, y_min, z_min, x_max, y_max, z_max)
    }

    /// Shared Ritter + Naive core used by `from_points`, `from_vertices`, and
    /// `from_encoded_cartesian_vertices`. `points` is the full list of positions and the
    /// `*_min`/`*_max` arguments are the extreme points along each axis.
    fn from_points_with_extremes(
        points: &[DVec3],
        x_min: DVec3,
        y_min: DVec3,
        z_min: DVec3,
        x_max: DVec3,
        y_max: DVec3,
        z_max: DVec3,
    ) -> Self {
        // Compute x-, y-, and z-spans (squared distances between each component's min and max).
        let x_span = (x_max - x_min).length_squared();
        let y_span = (y_max - y_min).length_squared();
        let z_span = (z_max - z_min).length_squared();

        // Set the diameter endpoints to the largest span.
        let mut diameter1 = x_min;
        let mut diameter2 = x_max;
        let mut max_span = x_span;
        if y_span > max_span {
            max_span = y_span;
            diameter1 = y_min;
            diameter2 = y_max;
        }
        if z_span > max_span {
            diameter1 = z_min;
            diameter2 = z_max;
        }

        // Initial sphere from Ritter's algorithm.
        let mut ritter_center = (diameter1 + diameter2) * 0.5;
        let mut radius_squared = (diameter2 - ritter_center).length_squared();
        let mut ritter_radius = radius_squared.sqrt();

        // Center of the sphere found using the Naive method.
        let min_box_pt = DVec3::new(x_min.x, y_min.y, z_min.z);
        let max_box_pt = DVec3::new(x_max.x, y_max.y, z_max.z);
        let naive_center = (min_box_pt + max_box_pt) * 0.5;

        // 2nd pass: find naive radius and modify the Ritter sphere to include all points.
        let mut naive_radius: f64 = 0.0;
        for &current_pos in points {
            let r = (current_pos - naive_center).length();
            if r > naive_radius {
                naive_radius = r;
            }

            let old_center_to_point_squared = (current_pos - ritter_center).length_squared();
            if old_center_to_point_squared > radius_squared {
                let old_center_to_point = old_center_to_point_squared.sqrt();
                ritter_radius = (ritter_radius + old_center_to_point) * 0.5;
                radius_squared = ritter_radius * ritter_radius;
                let old_to_new = old_center_to_point - ritter_radius;
                ritter_center = (ritter_center * ritter_radius + current_pos * old_to_new)
                    / old_center_to_point;
            }
        }

        if ritter_radius < naive_radius {
            Self {
                center: ritter_center,
                radius: ritter_radius,
            }
        } else {
            Self {
                center: naive_center,
                radius: naive_radius,
            }
        }
    }

    /// Computes a tight-fitting bounding sphere from points stored in a flat array
    /// (X, Y, Z order) with an optional relative center and stride.
    /// Maps to `BoundingSphere.fromVertices`
    pub fn from_vertices(vertices: &[f64], center: DVec3, stride: usize) -> Self {
        debug_assert!(stride >= 3, "stride must be at least 3");
        if vertices.is_empty() {
            return Self {
                center: DVec3::ZERO,
                radius: 0.0,
            };
        }

        let mut positions = Vec::new();
        let mut i = 0;
        while i + 2 < vertices.len() {
            positions.push(DVec3::new(
                vertices[i] + center.x,
                vertices[i + 1] + center.y,
                vertices[i + 2] + center.z,
            ));
            i += stride;
        }

        if positions.is_empty() {
            return Self {
                center: DVec3::ZERO,
                radius: 0.0,
            };
        }

        let mut x_min = positions[0];
        let mut y_min = positions[0];
        let mut z_min = positions[0];
        let mut x_max = positions[0];
        let mut y_max = positions[0];
        let mut z_max = positions[0];
        for &p in positions.iter().skip(1) {
            if p.x < x_min.x {
                x_min = p;
            }
            if p.x > x_max.x {
                x_max = p;
            }
            if p.y < y_min.y {
                y_min = p;
            }
            if p.y > y_max.y {
                y_max = p;
            }
            if p.z < z_min.z {
                z_min = p;
            }
            if p.z > z_max.z {
                z_max = p;
            }
        }

        Self::from_points_with_extremes(
            &positions, x_min, y_min, z_min, x_max, y_max, z_max,
        )
    }

    /// Computes a tight-fitting bounding sphere from encoded (high/low) flat arrays.
    /// Maps to `BoundingSphere.fromEncodedCartesianVertices`
    pub fn from_encoded_cartesian_vertices(positions_high: &[f64], positions_low: &[f64]) -> Self {
        if positions_high.len() != positions_low.len() || positions_high.is_empty() {
            return Self {
                center: DVec3::ZERO,
                radius: 0.0,
            };
        }

        let mut positions = Vec::new();
        let mut i = 0;
        while i + 2 < positions_high.len() {
            positions.push(DVec3::new(
                positions_high[i] + positions_low[i],
                positions_high[i + 1] + positions_low[i + 1],
                positions_high[i + 2] + positions_low[i + 2],
            ));
            i += 3;
        }

        if positions.is_empty() {
            return Self {
                center: DVec3::ZERO,
                radius: 0.0,
            };
        }

        let mut x_min = positions[0];
        let mut y_min = positions[0];
        let mut z_min = positions[0];
        let mut x_max = positions[0];
        let mut y_max = positions[0];
        let mut z_max = positions[0];
        for &p in positions.iter().skip(1) {
            if p.x < x_min.x {
                x_min = p;
            }
            if p.x > x_max.x {
                x_max = p;
            }
            if p.y < y_min.y {
                y_min = p;
            }
            if p.y > y_max.y {
                y_max = p;
            }
            if p.z < z_min.z {
                z_min = p;
            }
            if p.z > z_max.z {
                z_max = p;
            }
        }

        Self::from_points_with_extremes(
            &positions, x_min, y_min, z_min, x_max, y_max, z_max,
        )
    }

    /// Computes a bounding sphere from a rectangle projected in 2D.
    /// Maps to `BoundingSphere.fromRectangle2D`
    pub fn from_rectangle_2d(rectangle: &Rectangle, projection: &dyn MapProjection) -> Self {
        Self::from_rectangle_with_heights_2d(rectangle, projection, 0.0, 0.0)
    }

    /// Computes a bounding sphere from a rectangle projected in 2D, accounting for
    /// minimum and maximum heights.
    /// Maps to `BoundingSphere.fromRectangleWithHeights2D`
    pub fn from_rectangle_with_heights_2d(
        rectangle: &Rectangle,
        projection: &dyn MapProjection,
        minimum_height: f64,
        maximum_height: f64,
    ) -> Self {
        let mut southwest = rectangle.southwest();
        southwest.height = minimum_height;
        let mut northeast = rectangle.northeast();
        northeast.height = maximum_height;

        let lower_left = projection.project(&southwest);
        let upper_right = projection.project(&northeast);

        let width = upper_right.x - lower_left.x;
        let height = upper_right.y - lower_left.y;
        let elevation = upper_right.z - lower_left.z;

        Self {
            center: DVec3::new(
                lower_left.x + width * 0.5,
                lower_left.y + height * 0.5,
                lower_left.z + elevation * 0.5,
            ),
            radius: (width * width + height * height + elevation * elevation).sqrt() * 0.5,
        }
    }

    /// Computes a bounding sphere from a rectangle in 3D using a subsample of points.
    /// Maps to `BoundingSphere.fromRectangle3D`
    pub fn from_rectangle_3d(
        rectangle: &Rectangle,
        ellipsoid: &crate::ellipsoid::Ellipsoid,
        surface_height: f64,
    ) -> Self {
        let positions = rectangle.subsample(ellipsoid, surface_height);
        Self::from_points(&positions)
    }

    /// Computes a bounding sphere from the corner points of an axis-aligned box.
    /// Maps to `BoundingSphere.fromCornerPoints`
    pub fn from_corner_points(corner: DVec3, opposite_corner: DVec3) -> Self {
        let center = (corner + opposite_corner) * 0.5;
        let radius = center.distance(opposite_corner);
        Self { center, radius }
    }

    /// Creates a bounding sphere encompassing an ellipsoid.
    /// Maps to `BoundingSphere.fromEllipsoid`
    pub fn from_ellipsoid(ellipsoid: &crate::ellipsoid::Ellipsoid) -> Self {
        Self {
            center: DVec3::ZERO,
            radius: ellipsoid.maximum_radius(),
        }
    }

    /// Computes a tight-fitting bounding sphere enclosing the provided bounding spheres.
    /// Maps to `BoundingSphere.fromBoundingSpheres`
    pub fn from_bounding_spheres(spheres: &[BoundingSphere]) -> Self {
        if spheres.is_empty() {
            return Self {
                center: DVec3::ZERO,
                radius: 0.0,
            };
        }
        if spheres.len() == 1 {
            return spheres[0];
        }
        if spheres.len() == 2 {
            return spheres[0].union(&spheres[1]);
        }

        let positions: Vec<DVec3> = spheres.iter().map(|s| s.center).collect();
        let mut result = Self::from_points(&positions);
        let center = result.center;
        let mut radius = result.radius;
        for s in spheres {
            radius = radius.max(center.distance(s.center) + s.radius);
        }
        result.radius = radius;
        result
    }

    /// Computes a tight-fitting bounding sphere enclosing an affine transformation.
    /// Maps to `BoundingSphere.fromTransformation`
    pub fn from_transformation(transformation: &glam::DMat4) -> Self {
        let center = transformation.w_axis.truncate();
        let scale = DVec3::new(
            transformation.x_axis.truncate().length(),
            transformation.y_axis.truncate().length(),
            transformation.z_axis.truncate().length(),
        );
        let radius = 0.5 * scale.length();
        Self { center, radius }
    }

    /// Computes the distance from the closest point on the sphere to a point.
    /// (Non-squared convenience wrapper; CesiumJS exposes `distanceSquaredTo`.)
    pub fn distance_to(&self, point: DVec3) -> f64 {
        let dist = (point - self.center).length();
        (dist - self.radius).max(0.0)
    }

    /// Computes the estimated distance squared from the closest point on the sphere to a point.
    /// Maps to `BoundingSphere.distanceSquaredTo`
    pub fn distance_squared_to(&self, cartesian: DVec3) -> f64 {
        let distance = (self.center - cartesian).length() - self.radius;
        if distance <= 0.0 {
            0.0
        } else {
            distance * distance
        }
    }

    /// Determines if a point is inside the sphere.
    pub fn contains(&self, point: DVec3) -> bool {
        (point - self.center).length_squared() <= self.radius * self.radius
    }

    /// Computes the bounding sphere that contains both spheres.
    /// Maps to `BoundingSphere.union`
    pub fn union(&self, other: &Self) -> Self {
        let left_center = self.center;
        let left_radius = self.radius;
        let right_center = other.center;
        let right_radius = other.radius;

        let to_right_center = right_center - left_center;
        let center_separation = to_right_center.length();

        if left_radius >= center_separation + right_radius {
            // Left sphere wins.
            return *self;
        }
        if right_radius >= center_separation + left_radius {
            // Right sphere wins.
            return *other;
        }

        // Two tangent points, one on the far side of each sphere.
        let half_distance_between_tangent_points =
            (left_radius + center_separation + right_radius) * 0.5;
        let center = left_center
            + to_right_center
                * ((-left_radius + half_distance_between_tangent_points) / center_separation);

        Self {
            center,
            radius: half_distance_between_tangent_points,
        }
    }

    /// Enlarges the sphere to contain the provided point.
    /// Maps to `BoundingSphere.expand`
    pub fn expand(&self, point: DVec3) -> Self {
        let radius = (point - self.center).length();
        Self {
            center: self.center,
            radius: self.radius.max(radius),
        }
    }

    /// Determines which side of a plane the sphere is located.
    /// Maps to `BoundingSphere.intersectPlane`
    pub fn intersect_plane(&self, normal: DVec3, distance: f64) -> Intersect {
        let distance_to_plane = normal.dot(self.center) + distance;

        if distance_to_plane < -self.radius {
            Intersect::Outside
        } else if distance_to_plane < self.radius {
            Intersect::Intersecting
        } else {
            Intersect::Inside
        }
    }

    /// Applies a 4x4 affine transformation matrix to the sphere.
    /// Maps to `BoundingSphere.transform`
    pub fn transform(&self, matrix: &glam::DMat4) -> Self {
        let center = matrix.transform_point3(self.center);
        let scale_x = matrix.x_axis.truncate().length();
        let scale_y = matrix.y_axis.truncate().length();
        let scale_z = matrix.z_axis.truncate().length();
        let max_scale = scale_x.max(scale_y).max(scale_z);
        Self {
            center,
            radius: self.radius * max_scale,
        }
    }

    /// Applies a 4x4 transformation matrix assuming no scale.
    /// Maps to `BoundingSphere.transformWithoutScale`
    pub fn transform_without_scale(&self, matrix: &glam::DMat4) -> Self {
        Self {
            center: matrix.transform_point3(self.center),
            radius: self.radius,
        }
    }

    /// Computes the nearest and farthest distances from a position along a direction.
    /// Maps to `BoundingSphere.computePlaneDistances`
    pub fn compute_plane_distances(&self, position: DVec3, direction: DVec3) -> Interval {
        let to_center = self.center - position;
        let mag = direction.dot(to_center);
        Interval {
            start: mag - self.radius,
            stop: mag + self.radius,
        }
    }

    /// Computes the volume of the sphere.
    /// Maps to `BoundingSphere.prototype.volume`
    pub fn volume(&self) -> f64 {
        let radius = self.radius;
        (4.0 / 3.0) * std::f64::consts::PI * radius * radius * radius
    }
}

/// A numeric interval with a start and stop value.
/// Maps to CesiumJS `Interval`
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Interval {
    /// The start (minimum) value.
    pub start: f64,
    /// The stop (maximum) value.
    pub stop: f64,
}

impl Interval {
    pub fn new(start: f64, stop: f64) -> Self {
        Self { start, stop }
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

impl Default for OrientedBoundingBox {
    fn default() -> Self {
        Self {
            center: DVec3::ZERO,
            half_axes: DMat3::ZERO,
        }
    }
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

        // CesiumJS convention: plane normals point inward.
        if dist_to_center <= -rad_effective {
            Intersect::Outside
        } else if dist_to_center >= rad_effective {
            Intersect::Inside
        } else {
            Intersect::Intersecting
        }
    }

    /// Computes an OrientedBoundingBox of the given positions.
    ///
    /// This is an implementation of Stefan Gottschalk's Collision Queries using
    /// Oriented Bounding Boxes solution (PhD thesis): it builds the covariance
    /// matrix of the points, extracts its eigen-decomposition (classical Jacobi)
    /// to obtain the box orientation, then fits the extents along each eigen-axis.
    /// Maps to `OrientedBoundingBox.fromPoints`
    pub fn from_points(points: &[DVec3]) -> Self {
        if points.is_empty() {
            return Self {
                center: DVec3::ZERO,
                half_axes: DMat3::ZERO,
            };
        }

        let length = points.len();
        let inv_length = 1.0 / length as f64;

        let mut mean_point = points[0];
        for p in points.iter().skip(1) {
            mean_point += *p;
        }
        mean_point *= inv_length;

        let mut exx = 0.0;
        let mut exy = 0.0;
        let mut exz = 0.0;
        let mut eyy = 0.0;
        let mut eyz = 0.0;
        let mut ezz = 0.0;
        for p in points {
            let d = *p - mean_point;
            exx += d.x * d.x;
            exy += d.x * d.y;
            exz += d.x * d.z;
            eyy += d.y * d.y;
            eyz += d.y * d.z;
            ezz += d.z * d.z;
        }
        exx *= inv_length;
        exy *= inv_length;
        exz *= inv_length;
        eyy *= inv_length;
        eyz *= inv_length;
        ezz *= inv_length;

        // Column-major covariance matrix (matches CesiumJS flat-array layout).
        let covariance = DMat3::from_cols_array(&[
            exx, exy, exz, exy, eyy, eyz, exz, eyz, ezz,
        ]);

        let (unitary, _diagonal) = compute_eigen_decomposition(covariance);
        let rotation = unitary;

        let v1 = rotation.x_axis;
        let v2 = rotation.y_axis;
        let v3 = rotation.z_axis;

        let mut u1 = f64::MIN;
        let mut u2 = f64::MIN;
        let mut u3 = f64::MIN;
        let mut l1 = f64::MAX;
        let mut l2 = f64::MAX;
        let mut l3 = f64::MAX;
        for p in points {
            u1 = u1.max(v1.dot(*p));
            u2 = u2.max(v2.dot(*p));
            u3 = u3.max(v3.dot(*p));
            l1 = l1.min(v1.dot(*p));
            l2 = l2.min(v2.dot(*p));
            l3 = l3.min(v3.dot(*p));
        }

        let center = v1 * (0.5 * (l1 + u1)) + v2 * (0.5 * (l2 + u2)) + v3 * (0.5 * (l3 + u3));

        let scale = DVec3::new(u1 - l1, u2 - l2, u3 - l3) * 0.5;
        let half_axes = DMat3::from_cols(
            rotation.x_axis * scale.x,
            rotation.y_axis * scale.y,
            rotation.z_axis * scale.z,
        );

        Self { center, half_axes }
    }

    /// Computes an OrientedBoundingBox that bounds a `Rectangle` on the surface of an `Ellipsoid`.
    ///
    /// For rectangles no wider than half the ellipsoid (`width <= PI`) the box is aligned
    /// with the tangent plane at the rectangle center; wider rectangles use a plane that
    /// rotates about the Z axis. Maps to `OrientedBoundingBox.fromRectangle`
    ///
    /// # Panics
    /// Mirrors the CesiumJS debug-only `DeveloperError` checks (via `debug_assert!`):
    /// `rectangle.width` must be in `[0, 2*PI]`, `rectangle.height` in `[0, PI]`, and the
    /// ellipsoid must be an ellipsoid of revolution (`radii.x == radii.y`).
    pub fn from_rectangle(
        rectangle: &Rectangle,
        minimum_height: f64,
        maximum_height: f64,
        ellipsoid: &Ellipsoid,
    ) -> Self {
        debug_assert!(
            rectangle.width() >= 0.0 && rectangle.width() <= TWO_PI,
            "Rectangle width must be between 0 and 2 * pi"
        );
        debug_assert!(
            rectangle.height() >= 0.0 && rectangle.height() <= PI_F64,
            "Rectangle height must be between 0 and pi"
        );
        debug_assert!(
            math_utils::equals_epsilon(
                ellipsoid.radii().x,
                ellipsoid.radii().y,
                EPSILON15,
                EPSILON15
            ),
            "Ellipsoid must be an ellipsoid of revolution (radii.x == radii.y)"
        );

        if rectangle.width() <= PI_F64 {
            // The bounding box will be aligned with the tangent plane at the center of the rectangle.
            let tangent_point_cartographic = rectangle.center();
            let tangent_point = ellipsoid.cartographic_to_cartesian(&tangent_point_cartographic);
            let (tp_origin, x_axis, y_axis, z_axis) = tangent_plane_frame(tangent_point, ellipsoid);
            let plane = Plane::from_point_normal(tp_origin, z_axis);

            // If the rectangle spans the equator, CW is instead aligned with the equator
            // (because it sticks out the farthest at the equator).
            let lon_center = tangent_point_cartographic.longitude;
            let lat_center = if rectangle.south < 0.0 && rectangle.north > 0.0 {
                0.0
            } else {
                tangent_point_cartographic.latitude
            };

            // Compute XY extents using the rectangle at maximum height.
            let nc = ellipsoid.cartographic_to_cartesian(&Cartographic::from_radians(
                lon_center,
                rectangle.north,
                maximum_height,
            ));
            let nw = ellipsoid.cartographic_to_cartesian(&Cartographic::from_radians(
                rectangle.west,
                rectangle.north,
                maximum_height,
            ));
            let cw = ellipsoid.cartographic_to_cartesian(&Cartographic::from_radians(
                rectangle.west,
                lat_center,
                maximum_height,
            ));
            let sw = ellipsoid.cartographic_to_cartesian(&Cartographic::from_radians(
                rectangle.west,
                rectangle.south,
                maximum_height,
            ));
            let sc = ellipsoid.cartographic_to_cartesian(&Cartographic::from_radians(
                lon_center,
                rectangle.south,
                maximum_height,
            ));

            let p_nc = project_to_nearest(tp_origin, x_axis, y_axis, z_axis, nc);
            let p_nw = project_to_nearest(tp_origin, x_axis, y_axis, z_axis, nw);
            let p_cw = project_to_nearest(tp_origin, x_axis, y_axis, z_axis, cw);
            let p_sw = project_to_nearest(tp_origin, x_axis, y_axis, z_axis, sw);
            let p_sc = project_to_nearest(tp_origin, x_axis, y_axis, z_axis, sc);

            let min_x = p_nw.x.min(p_cw.x).min(p_sw.x);
            let max_x = -min_x; // symmetrical

            let max_y = p_nw.y.max(p_nc.y);
            let min_y = p_sw.y.min(p_sc.y);

            // Compute minimum Z using the rectangle at minimum height, since it will be
            // deeper than the maximum height.
            let nw_low = ellipsoid.cartographic_to_cartesian(&Cartographic::from_radians(
                rectangle.west,
                rectangle.north,
                minimum_height,
            ));
            let sw_low = ellipsoid.cartographic_to_cartesian(&Cartographic::from_radians(
                rectangle.west,
                rectangle.south,
                minimum_height,
            ));
            let min_z = plane.point_distance(nw_low).min(plane.point_distance(sw_low));
            let max_z = maximum_height; // tangent plane touches the surface at height = 0

            return from_plane_extents(
                tp_origin, x_axis, y_axis, z_axis, min_x, max_x, min_y, max_y, min_z, max_z,
            );
        }

        // Handle the case where rectangle width is greater than PI (wraps around more than
        // half the ellipsoid).
        let fully_above_equator = rectangle.south > 0.0;
        let fully_below_equator = rectangle.north < 0.0;
        let latitude_nearest_to_equator = if fully_above_equator {
            rectangle.south
        } else if fully_below_equator {
            rectangle.north
        } else {
            0.0
        };
        let center_longitude = rectangle.center().longitude;

        // Plane is located at the rectangle's center longitude and the rectangle's latitude
        // that is closest to the equator. It rotates around the Z axis.
        let mut plane_origin = ellipsoid.cartographic_to_cartesian(&Cartographic::from_radians(
            center_longitude,
            latitude_nearest_to_equator,
            maximum_height,
        ));
        plane_origin.z = 0.0; // center the plane on the equator to simplify plane normal calculation
        let is_pole = plane_origin.x.abs() < EPSILON10 && plane_origin.y.abs() < EPSILON10;
        let plane_normal = if !is_pole {
            plane_origin.normalize()
        } else {
            DVec3::X
        };
        let plane_y_axis = DVec3::Z;
        let plane_x_axis = plane_normal.cross(plane_y_axis);
        let plane = Plane::from_point_normal(plane_origin, plane_normal);

        // Get the horizon point relative to the center. This will be the farthest extent in
        // the plane's X dimension.
        let horizon_cartesian = ellipsoid.cartographic_to_cartesian(&Cartographic::from_radians(
            center_longitude + math_utils::PI_OVER_TWO,
            latitude_nearest_to_equator,
            maximum_height,
        ));
        let max_x = plane
            .project_point_onto_plane(horizon_cartesian)
            .dot(plane_x_axis);
        let min_x = -max_x; // symmetrical

        // Get the min and max Y, using the height that will give the largest extent.
        let max_y = ellipsoid
            .cartographic_to_cartesian(&Cartographic::from_radians(
                0.0,
                rectangle.north,
                if fully_below_equator {
                    minimum_height
                } else {
                    maximum_height
                },
            ))
            .z;
        let min_y = ellipsoid
            .cartographic_to_cartesian(&Cartographic::from_radians(
                0.0,
                rectangle.south,
                if fully_above_equator {
                    minimum_height
                } else {
                    maximum_height
                },
            ))
            .z;

        let far_z = ellipsoid.cartographic_to_cartesian(&Cartographic::from_radians(
            rectangle.east,
            latitude_nearest_to_equator,
            maximum_height,
        ));
        let min_z = plane.point_distance(far_z);
        let max_z = 0.0; // plane origin starts at maxZ already

        // min and max are local to the plane axes
        from_plane_extents(
            plane_origin,
            plane_x_axis,
            plane_y_axis,
            plane_normal,
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z,
        )
    }

    /// Computes an OrientedBoundingBox that bounds an affine transformation.
    /// Maps to `OrientedBoundingBox.fromTransformation`
    pub fn from_transformation(transformation: &DMat4) -> Self {
        let center = transformation.w_axis.truncate();
        let half_axes = DMat3::from_cols(
            transformation.x_axis.truncate(),
            transformation.y_axis.truncate(),
            transformation.z_axis.truncate(),
        ) * 0.5;
        Self { center, half_axes }
    }

    /// Computes the estimated distance squared from the closest point on the box to a point.
    /// Returns 0 if the point is inside the box.
    ///
    /// Faithfully ports the degenerate-axis handling (one/two/three zero-length half-axes)
    /// from CesiumJS. Maps to `OrientedBoundingBox.distanceSquaredTo`
    pub fn distance_squared_to(&self, cartesian: DVec3) -> f64 {
        // See Geometric Tools for Computer Graphics 10.4.2
        let offset = cartesian - self.center;

        let mut u = self.half_axes.x_axis;
        let mut v = self.half_axes.y_axis;
        let mut w = self.half_axes.z_axis;

        let u_half = u.length();
        let v_half = v.length();
        let w_half = w.length();

        let mut u_valid = true;
        let mut v_valid = true;
        let mut w_valid = true;

        if u_half > 0.0 {
            u /= u_half;
        } else {
            u_valid = false;
        }
        if v_half > 0.0 {
            v /= v_half;
        } else {
            v_valid = false;
        }
        if w_half > 0.0 {
            w /= w_half;
        } else {
            w_valid = false;
        }

        let number_of_degenerate_axes =
            (!u_valid as u8) + (!v_valid as u8) + (!w_valid as u8);

        if number_of_degenerate_axes == 1 {
            let mut degenerate_axis = u;
            let mut valid_axis1 = v;
            let mut valid_axis2 = w;
            if !v_valid {
                degenerate_axis = v;
                valid_axis1 = u;
            } else if !w_valid {
                degenerate_axis = w;
                valid_axis2 = u;
            }

            let valid_axis3 = valid_axis1.cross(valid_axis2);

            if degenerate_axis == u {
                u = valid_axis3;
            } else if degenerate_axis == v {
                v = valid_axis3;
            } else if degenerate_axis == w {
                w = valid_axis3;
            }
        } else if number_of_degenerate_axes == 2 {
            let mut valid_axis1 = u;
            let mut valid_axis1_is = 0u8; // 0 => u, 1 => v, 2 => w
            if v_valid {
                valid_axis1 = v;
                valid_axis1_is = 1;
            } else if w_valid {
                valid_axis1 = w;
                valid_axis1_is = 2;
            }

            let mut cross_vector = DVec3::Y;
            if cross_vector.abs_diff_eq(valid_axis1, math_utils::EPSILON3) {
                cross_vector = DVec3::X;
            }

            let mut valid_axis2 = valid_axis1.cross(cross_vector);
            valid_axis2 = valid_axis2.normalize();
            let mut valid_axis3 = valid_axis1.cross(valid_axis2);
            valid_axis3 = valid_axis3.normalize();

            match valid_axis1_is {
                0 => {
                    v = valid_axis2;
                    w = valid_axis3;
                }
                1 => {
                    w = valid_axis2;
                    u = valid_axis3;
                }
                _ => {
                    u = valid_axis2;
                    v = valid_axis3;
                }
            }
        } else if number_of_degenerate_axes == 3 {
            u = DVec3::X;
            v = DVec3::Y;
            w = DVec3::Z;
        }

        let p_prime = DVec3::new(offset.dot(u), offset.dot(v), offset.dot(w));

        let mut distance_squared = 0.0;
        let mut d;

        if p_prime.x < -u_half {
            d = p_prime.x + u_half;
            distance_squared += d * d;
        } else if p_prime.x > u_half {
            d = p_prime.x - u_half;
            distance_squared += d * d;
        }

        if p_prime.y < -v_half {
            d = p_prime.y + v_half;
            distance_squared += d * d;
        } else if p_prime.y > v_half {
            d = p_prime.y - v_half;
            distance_squared += d * d;
        }

        if p_prime.z < -w_half {
            d = p_prime.z + w_half;
            distance_squared += d * d;
        } else if p_prime.z > w_half {
            d = p_prime.z - w_half;
            distance_squared += d * d;
        }

        distance_squared
    }

    /// Computes the nearest and farthest distances, along `direction` from `position`,
    /// to the planes that intersect the bounding box.
    /// Maps to `OrientedBoundingBox.computePlaneDistances`
    pub fn compute_plane_distances(&self, position: DVec3, direction: DVec3) -> Interval {
        let mut min_dist = f64::INFINITY;
        let mut max_dist = f64::NEG_INFINITY;

        let center = self.center;
        let u = self.half_axes.x_axis;
        let v = self.half_axes.y_axis;
        let w = self.half_axes.z_axis;

        let signs: [(f64, f64, f64); 8] = [
            (1.0, 1.0, 1.0),
            (1.0, 1.0, -1.0),
            (1.0, -1.0, 1.0),
            (1.0, -1.0, -1.0),
            (-1.0, 1.0, 1.0),
            (-1.0, 1.0, -1.0),
            (-1.0, -1.0, 1.0),
            (-1.0, -1.0, -1.0),
        ];

        for (su, sv, sw) in signs {
            let corner = center + u * su + v * sv + w * sw;
            let to_center = corner - position;
            let mag = direction.dot(to_center);
            min_dist = min_dist.min(mag);
            max_dist = max_dist.max(mag);
        }

        Interval::new(min_dist, max_dist)
    }

    /// Computes the eight corners of the box, ordered by
    /// `(-X,-Y,-Z), (-X,-Y,+Z), (-X,+Y,-Z), (-X,+Y,+Z), (+X,-Y,-Z), (+X,-Y,+Z), (+X,+Y,-Z), (+X,+Y,+Z)`.
    /// Maps to `OrientedBoundingBox.computeCorners`
    pub fn compute_corners(&self) -> [DVec3; 8] {
        let center = self.center;
        let x_axis = self.half_axes.x_axis;
        let y_axis = self.half_axes.y_axis;
        let z_axis = self.half_axes.z_axis;

        [
            center - x_axis - y_axis - z_axis,
            center - x_axis - y_axis + z_axis,
            center - x_axis + y_axis - z_axis,
            center - x_axis + y_axis + z_axis,
            center + x_axis - y_axis - z_axis,
            center + x_axis - y_axis + z_axis,
            center + x_axis + y_axis - z_axis,
            center + x_axis + y_axis + z_axis,
        ]
    }

    /// Computes a transformation matrix (a `DMat4`) from the oriented bounding box:
    /// a uniform scale of 2 applied to the half-axes, plus the center as translation.
    /// Maps to `OrientedBoundingBox.computeTransformation`
    pub fn compute_transformation(&self) -> DMat4 {
        let rotation_scale = self.half_axes * 2.0;
        DMat4::from_cols(
            rotation_scale.x_axis.extend(0.0),
            rotation_scale.y_axis.extend(0.0),
            rotation_scale.z_axis.extend(0.0),
            self.center.extend(1.0),
        )
    }
}

/// Builds the tangent-plane frame `(origin, x_axis, y_axis, z_axis)` for an ellipsoid at a
/// point, mirroring CesiumJS `EllipsoidTangentPlane` (origin projected to the geodetic
/// surface, axes taken from the East-North-Up frame).
fn tangent_plane_frame(origin: DVec3, ellipsoid: &Ellipsoid) -> (DVec3, DVec3, DVec3, DVec3) {
    let origin = ellipsoid
        .scale_to_geodetic_surface(origin)
        .expect("origin must not be at the center of the ellipsoid");
    let enu = crate::transforms::east_north_up_to_fixed_frame(origin, ellipsoid);
    let x_axis = enu.x_axis.truncate();
    let y_axis = enu.y_axis.truncate();
    let z_axis = enu.z_axis.truncate();
    (origin, x_axis, y_axis, z_axis)
}

/// Projects a 3D point onto the tangent plane along the plane normal, returning local 2D
/// coordinates. Mirrors CesiumJS `EllipsoidTangentPlane.projectPointToNearestOnPlane`.
fn project_to_nearest(
    origin: DVec3,
    x_axis: DVec3,
    y_axis: DVec3,
    normal: DVec3,
    cartesian: DVec3,
) -> DVec2 {
    let plane = Plane::from_point_normal(origin, normal);
    let ray = Ray::new(cartesian, normal);
    let mut intersection = ray_plane(&ray, &plane);
    if intersection.is_none() {
        let ray = Ray::new(cartesian, -normal);
        intersection = ray_plane(&ray, &plane);
    }
    let intersection = intersection.expect("tangent plane projection must intersect");
    let v = intersection - origin;
    DVec2::new(x_axis.dot(v), y_axis.dot(v))
}

/// Builds an OrientedBoundingBox from a plane origin/axes and local min/max extents.
/// Mirrors CesiumJS `fromPlaneExtents`.
#[allow(clippy::too_many_arguments)]
fn from_plane_extents(
    plane_origin: DVec3,
    plane_x_axis: DVec3,
    plane_y_axis: DVec3,
    plane_z_axis: DVec3,
    minimum_x: f64,
    maximum_x: f64,
    minimum_y: f64,
    maximum_y: f64,
    minimum_z: f64,
    maximum_z: f64,
) -> OrientedBoundingBox {
    let half_axes = DMat3::from_cols(plane_x_axis, plane_y_axis, plane_z_axis);

    let center_offset = DVec3::new(
        (minimum_x + maximum_x) / 2.0,
        (minimum_y + maximum_y) / 2.0,
        (minimum_z + maximum_z) / 2.0,
    );
    let scale = DVec3::new(
        (maximum_x - minimum_x) / 2.0,
        (maximum_y - minimum_y) / 2.0,
        (maximum_z - minimum_z) / 2.0,
    );

    let center = plane_origin + half_axes * center_offset;
    let half_axes = DMat3::from_cols(
        half_axes.x_axis * scale.x,
        half_axes.y_axis * scale.y,
        half_axes.z_axis * scale.z,
    );

    OrientedBoundingBox {
        center,
        half_axes,
    }
}

// --- Matrix3 eigen decomposition (classical Jacobi algorithm) ---
// Maps to CesiumJS `Matrix3.computeEigenDecomposition` (Golub & Van Loan, 3rd ed., 8.4.3)
// and its helpers `computeFrobeniusNorm`, `offDiagonalFrobeniusNorm`, `shurDecomposition`.
// The flat indexing `[col * 3 + row]` matches CesiumJS `Matrix3.getElementIndex(col, row)`.

#[inline]
fn frobenius_norm(m: &[f64; 9]) -> f64 {
    let mut norm = 0.0;
    for i in 0..9 {
        norm += m[i] * m[i];
    }
    norm.sqrt()
}

// Off-diagonal pairs (col, row): (2,1), (2,0), (1,0) — matches CesiumJS colVal/rowVal.
const EIGEN_COL_VAL: [usize; 3] = [2, 2, 1];
const EIGEN_ROW_VAL: [usize; 3] = [1, 0, 0];

#[inline]
fn off_diagonal_frobenius_norm(m: &[f64; 9]) -> f64 {
    let mut norm = 0.0;
    for i in 0..3 {
        let temp = m[EIGEN_COL_VAL[i] * 3 + EIGEN_ROW_VAL[i]];
        norm += 2.0 * temp * temp;
    }
    norm.sqrt()
}

/// 2-by-2 symmetric Schur decomposition (Golub & Van Loan 8.4.2). Returns the Jacobi
/// rotation matrix that reduces the largest off-diagonal term of `matrix`.
fn shur_decomposition(matrix: &[f64; 9]) -> [f64; 9] {
    let tolerance = math_utils::EPSILON15;

    let mut max_diagonal = 0.0;
    let mut rot_axis = 1usize;
    for i in 0..3 {
        let temp = matrix[EIGEN_COL_VAL[i] * 3 + EIGEN_ROW_VAL[i]].abs();
        if temp > max_diagonal {
            rot_axis = i;
            max_diagonal = temp;
        }
    }

    let mut c = 1.0;
    let mut s = 0.0;

    let p = EIGEN_ROW_VAL[rot_axis];
    let q = EIGEN_COL_VAL[rot_axis];

    if matrix[q * 3 + p].abs() > tolerance {
        let qq = matrix[q * 3 + q];
        let pp = matrix[p * 3 + p];
        let qp = matrix[q * 3 + p];

        let tau = (qq - pp) / 2.0 / qp;
        let t = if tau < 0.0 {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        } else {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        };

        c = 1.0 / (1.0 + t * t).sqrt();
        s = t * c;
    }

    // Identity with the (p, q) Givens rotation applied.
    let mut result = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    result[p * 3 + p] = c;
    result[q * 3 + q] = c;
    result[q * 3 + p] = s;
    result[p * 3 + q] = -s;
    result
}

/// Computes the eigen decomposition of a symmetric 3x3 matrix, returning
/// `(unitary, diagonal)` such that `matrix = unitary * diagonal * unitary^T`.
/// Maps to `Matrix3.computeEigenDecomposition`.
fn compute_eigen_decomposition(matrix: DMat3) -> (DMat3, DMat3) {
    let tolerance = EPSILON20;
    let max_sweeps = 10;

    let mut count = 0;
    let mut sweep = 0;

    let mut unitary = DMat3::IDENTITY;
    let mut diag = matrix;

    let epsilon = tolerance * frobenius_norm(&diag.to_cols_array());

    while sweep < max_sweeps && off_diagonal_frobenius_norm(&diag.to_cols_array()) > epsilon {
        let j_matrix = DMat3::from_cols_array(&shur_decomposition(&diag.to_cols_array()));
        let j_matrix_transpose = j_matrix.transpose();
        diag = diag * j_matrix;
        diag = j_matrix_transpose * diag;
        unitary = unitary * j_matrix;

        count += 1;
        if count > 2 {
            sweep += 1;
            count = 0;
        }
    }

    (unitary, diag)
}

/// An axis-aligned bounding box defined by minimum and maximum corners.
/// Maps to CesiumJS `AxisAlignedBoundingBox`
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct AxisAlignedBoundingBox {
    /// The minimum corner.
    pub minimum: DVec3,
    /// The maximum corner.
    pub maximum: DVec3,
    /// The center (computed).
    pub center: DVec3,
}

impl AxisAlignedBoundingBox {
    /// Creates an AABB from minimum/maximum corners, computing the center as the midpoint.
    /// Maps to the CesiumJS constructor `new AxisAlignedBoundingBox(minimum, maximum)`
    /// and `AxisAlignedBoundingBox.fromCorners`.
    pub fn new(minimum: DVec3, maximum: DVec3) -> Self {
        let center = (minimum + maximum) * 0.5;
        Self {
            minimum,
            maximum,
            center,
        }
    }

    /// Creates an AABB from its corners.
    /// Maps to `AxisAlignedBoundingBox.fromCorners`
    pub fn from_corners(minimum: DVec3, maximum: DVec3) -> Self {
        Self::new(minimum, maximum)
    }

    /// Creates an AABB with an explicit center.
    /// Maps to the CesiumJS constructor `new AxisAlignedBoundingBox(minimum, maximum, center)`
    pub fn with_center(minimum: DVec3, maximum: DVec3, center: DVec3) -> Self {
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

        // CesiumJS convention: plane normals point inward.
        if center_dist - rad_effective > 0.0 {
            Intersect::Inside
        } else if center_dist + rad_effective < 0.0 {
            Intersect::Outside
        } else {
            Intersect::Intersecting
        }
    }
}

/// A bounding rectangle given by a corner, width and height.
/// Maps to CesiumJS `BoundingRectangle`
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BoundingRectangle {
    /// The x coordinate of the rectangle (lower-left corner).
    pub x: f64,
    /// The y coordinate of the rectangle (lower-left corner).
    pub y: f64,
    /// The width of the rectangle.
    pub width: f64,
    /// The height of the rectangle.
    pub height: f64,
}

impl BoundingRectangle {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Computes a bounding rectangle enclosing a list of 2D points.
    /// Maps to `BoundingRectangle.fromPoints`
    pub fn from_points(points: &[DVec2]) -> Self {
        if points.is_empty() {
            return Self::default();
        }

        let mut minimum_x = points[0].x;
        let mut minimum_y = points[0].y;
        let mut maximum_x = points[0].x;
        let mut maximum_y = points[0].y;

        for p in points.iter().skip(1) {
            minimum_x = minimum_x.min(p.x);
            maximum_x = maximum_x.max(p.x);
            minimum_y = minimum_y.min(p.y);
            maximum_y = maximum_y.max(p.y);
        }

        Self {
            x: minimum_x,
            y: minimum_y,
            width: maximum_x - minimum_x,
            height: maximum_y - minimum_y,
        }
    }

    /// Computes a bounding rectangle from a geographic rectangle via a projection.
    /// Maps to `BoundingRectangle.fromRectangle`
    pub fn from_rectangle(rectangle: &Rectangle, projection: &dyn MapProjection) -> Self {
        let lower_left = projection.project(&rectangle.southwest());
        let upper_right = projection.project(&rectangle.northeast());

        Self {
            x: lower_left.x,
            y: lower_left.y,
            width: upper_right.x - lower_left.x,
            height: upper_right.y - lower_left.y,
        }
    }

    /// Computes the union of two bounding rectangles.
    /// Maps to `BoundingRectangle.union`
    pub fn union(&self, other: &Self) -> Self {
        let lower_left_x = self.x.min(other.x);
        let lower_left_y = self.y.min(other.y);
        let upper_right_x = (self.x + self.width).max(other.x + other.width);
        let upper_right_y = (self.y + self.height).max(other.y + other.height);

        Self {
            x: lower_left_x,
            y: lower_left_y,
            width: upper_right_x - lower_left_x,
            height: upper_right_y - lower_left_y,
        }
    }

    /// Enlarges the rectangle until it contains the given point.
    /// Maps to `BoundingRectangle.expand`
    pub fn expand(&self, point: DVec2) -> Self {
        let mut result = *self;

        let width = point.x - result.x;
        let height = point.y - result.y;

        if width > result.width {
            result.width = width;
        } else if width < 0.0 {
            result.width -= width;
            result.x = point.x;
        }

        if height > result.height {
            result.height = height;
        } else if height < 0.0 {
            result.height -= height;
            result.y = point.y;
        }

        result
    }

    /// Determines if two bounding rectangles intersect.
    /// Maps to `BoundingRectangle.intersect`
    pub fn intersect(&self, other: &Self) -> Intersect {
        let left_x = self.x;
        let left_y = self.y;
        let right_x = other.x;
        let right_y = other.y;

        if !(left_x > right_x + other.width
            || left_x + self.width < right_x
            || left_y + self.height < right_y
            || left_y > right_y + other.height)
        {
            Intersect::Intersecting
        } else {
            Intersect::Outside
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
