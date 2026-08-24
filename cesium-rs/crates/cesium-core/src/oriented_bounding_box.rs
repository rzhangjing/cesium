//! Ported from `packages/engine/Source/Core/OrientedBoundingBox.js`.

use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::check;
use crate::developer_error::throw_developer_error;
use crate::ellipsoid::Ellipsoid;
use crate::ellipsoid_tangent_plane::EllipsoidTangentPlane;
use crate::intersect::Intersect;
use crate::interval::Interval;
use crate::math::CesiumMath;
use crate::matrix3::Matrix3;
use crate::matrix4::Matrix4;
use crate::plane::Plane;
use crate::rectangle::Rectangle;

/// A closed and convex rectangular cuboid.
#[derive(Clone, Debug)]
pub struct OrientedBoundingBox {
    /// The center of the box.
    pub center: Cartesian3,
    /// The three orthogonal half-axes of the bounding box.
    pub half_axes: Matrix3,
}

impl Default for OrientedBoundingBox {
    fn default() -> Self {
        Self {
            center: Cartesian3::ZERO,
            half_axes: Matrix3::ZERO,
        }
    }
}

impl OrientedBoundingBox {
    /// Creates a new OrientedBoundingBox.
    pub fn new(center: Option<&Cartesian3>, half_axes: Option<&Matrix3>) -> Self {
        Self {
            center: center.copied().unwrap_or(Cartesian3::ZERO),
            half_axes: half_axes.copied().unwrap_or(Matrix3::ZERO),
        }
    }

    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize =
        Cartesian3::PACKED_LENGTH + Matrix3::PACKED_LENGTH;

    /// Stores the provided instance into the provided array.
    pub fn pack(value: &Self, array: &mut [f64], starting_index: Option<usize>) {
        let idx = starting_index.unwrap_or(0);
        Cartesian3::pack(&value.center, array, Some(idx));
        Matrix3::pack(&value.half_axes, array, idx + Cartesian3::PACKED_LENGTH);
    }

    /// Retrieves an instance from a packed array.
    pub fn unpack(
        array: &[f64],
        starting_index: Option<usize>,
        result: Option<&mut Self>,
    ) -> Self {
        let idx = starting_index.unwrap_or(0);
        let mut r = result.cloned().unwrap_or_default();
        Cartesian3::unpack(array, Some(idx), &mut r.center);
        Matrix3::unpack(array, idx + Cartesian3::PACKED_LENGTH, &mut r.half_axes);
        r
    }

    /// Computes an OrientedBoundingBox of the given positions.
    /// Implementation of Stefan Gottschalk's Collision Queries using OBB.
    pub fn from_points(positions: Option<&[Cartesian3]>, result: Option<&mut Self>) -> Self {
        let mut r = result.map(|r| { *r = Self::default(); r }).cloned()
            .unwrap_or_default();

        let positions = match positions {
            Some(p) if !p.is_empty() => p,
            _ => {
                r.half_axes = Matrix3::ZERO;
                r.center = Cartesian3::ZERO;
                return r;
            }
        };

        let length = positions.len();
        let inv_length = 1.0 / length as f64;

        // Compute mean
        let mut mean_point = positions[0];
        for i in 1..length {
            let mut sum = Cartesian3::ZERO;
            Cartesian3::add(&mean_point, &positions[i], &mut sum);
            mean_point = sum;
        }
        let mut scaled_mean = Cartesian3::ZERO;
        Cartesian3::multiply_by_scalar(&mean_point, inv_length, &mut scaled_mean);
        mean_point = scaled_mean;

        // Compute covariance matrix
        let mut exx = 0.0;
        let mut exy = 0.0;
        let mut exz = 0.0;
        let mut eyy = 0.0;
        let mut eyz = 0.0;
        let mut ezz = 0.0;

        for i in 0..length {
            let mut p = Cartesian3::ZERO;
            Cartesian3::subtract(&positions[i], &mean_point, &mut p);
            exx += p.x * p.x;
            exy += p.x * p.y;
            exz += p.x * p.z;
            eyy += p.y * p.y;
            eyz += p.y * p.z;
            ezz += p.z * p.z;
        }

        exx *= inv_length;
        exy *= inv_length;
        exz *= inv_length;
        eyy *= inv_length;
        eyz *= inv_length;
        ezz *= inv_length;

        let covariance_matrix = Matrix3::new(
            exx, exy, exz,
            exy, eyy, eyz,
            exz, eyz, ezz,
        );

        let eigen = Matrix3::compute_eigen_decomposition(&covariance_matrix, None);
        let rotation = eigen.unitary;

        let v1 = Matrix3::get_column_new(&rotation, 0);
        let v2 = Matrix3::get_column_new(&rotation, 1);
        let v3 = Matrix3::get_column_new(&rotation, 2);

        let mut u1 = f64::NEG_INFINITY;
        let mut u2 = f64::NEG_INFINITY;
        let mut u3 = f64::NEG_INFINITY;
        let mut l1 = f64::INFINITY;
        let mut l2 = f64::INFINITY;
        let mut l3 = f64::INFINITY;

        for i in 0..length {
            let p = &positions[i];
            u1 = Cartesian3::dot(&v1, p).max(u1);
            u2 = Cartesian3::dot(&v2, p).max(u2);
            u3 = Cartesian3::dot(&v3, p).max(u3);

            l1 = Cartesian3::dot(&v1, p).min(l1);
            l2 = Cartesian3::dot(&v2, p).min(l2);
            l3 = Cartesian3::dot(&v3, p).min(l3);
        }

        let mut v1s = Cartesian3::ZERO;
        Cartesian3::multiply_by_scalar(&v1, 0.5 * (l1 + u1), &mut v1s);
        let mut v2s = Cartesian3::ZERO;
        Cartesian3::multiply_by_scalar(&v2, 0.5 * (l2 + u2), &mut v2s);
        let mut v3s = Cartesian3::ZERO;
        Cartesian3::multiply_by_scalar(&v3, 0.5 * (l3 + u3), &mut v3s);

        let mut center = Cartesian3::ZERO;
        Cartesian3::add(&v1s, &v2s, &mut center);
        let mut center2 = Cartesian3::ZERO;
        Cartesian3::add(&center, &v3s, &mut center2);
        r.center = center2;

        let scale = Cartesian3::new(u1 - l1, u2 - l2, u3 - l3);
        let mut half_scale = Cartesian3::ZERO;
        Cartesian3::multiply_by_scalar(&scale, 0.5, &mut half_scale);
        Matrix3::multiply_by_scale(&rotation, &half_scale, &mut r.half_axes);

        r
    }

    /// Computes an OrientedBoundingBox that bounds a [`Rectangle`] on the
    /// surface of an [`Ellipsoid`]. There are no guarantees about the
    /// orientation of the bounding box.
    ///
    /// Port of `OrientedBoundingBox.fromRectangle`.
    pub fn from_rectangle(
        rectangle: Option<&Rectangle>,
        minimum_height: Option<f64>,
        maximum_height: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
        result: Option<&mut Self>,
    ) -> Self {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            check::defined("rectangle", rectangle);
            let rectangle = rectangle.unwrap();
            if rectangle.width() < 0.0 || rectangle.width() > CesiumMath::TWO_PI {
                throw_developer_error("Rectangle width must be between 0 and 2 * pi");
            }
            if rectangle.height() < 0.0 || rectangle.height() > CesiumMath::PI {
                throw_developer_error("Rectangle height must be between 0 and pi");
            }
            if let Some(ellipsoid) = &ellipsoid {
                if !CesiumMath::equals_epsilon(
                    ellipsoid.radii().x,
                    ellipsoid.radii().y,
                    Some(CesiumMath::EPSILON15),
                    None,
                ) {
                    throw_developer_error(
                        "Ellipsoid must be an ellipsoid of revolution (radii.x == radii.y)",
                    );
                }
            }
        }
        //>>includeEnd('debug');

        let rectangle = rectangle.unwrap();
        let minimum_height = minimum_height.unwrap_or(0.0);
        let maximum_height = maximum_height.unwrap_or(0.0);
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);

        let mut out = OrientedBoundingBox::default();

        if rectangle.width() <= CesiumMath::PI {
            // The bounding box will be aligned with the tangent plane at the
            // center of the rectangle.
            let tangent_point_cartographic = Rectangle::center(rectangle);
            let mut tangent_point = Cartesian3::default();
            ellipsoid.cartographic_to_cartesian(&tangent_point_cartographic, &mut tangent_point);
            let tangent_plane = EllipsoidTangentPlane::new(&tangent_point, Some(ellipsoid))
                .unwrap_or_else(|| {
                    throw_developer_error("the origin must not be at the center of the ellipsoid")
                });
            let plane = tangent_plane.plane().clone();

            // If the rectangle spans the equator, CW is instead aligned with
            // the equator (because it sticks out the farthest at the equator).
            let lon_center = tangent_point_cartographic.longitude;
            let lat_center = if rectangle.south < 0.0 && rectangle.north > 0.0 {
                0.0
            } else {
                tangent_point_cartographic.latitude
            };

            // Compute XY extents using the rectangle at maximum height
            let perimeter_cartographic_nc =
                Cartographic::from_radians_new(lon_center, rectangle.north, Some(maximum_height));
            let perimeter_cartographic_nw = Cartographic::from_radians_new(
                rectangle.west,
                rectangle.north,
                Some(maximum_height),
            );
            let perimeter_cartographic_cw =
                Cartographic::from_radians_new(rectangle.west, lat_center, Some(maximum_height));
            let perimeter_cartographic_sw = Cartographic::from_radians_new(
                rectangle.west,
                rectangle.south,
                Some(maximum_height),
            );
            let perimeter_cartographic_sc =
                Cartographic::from_radians_new(lon_center, rectangle.south, Some(maximum_height));

            let mut perimeter_cartesian_nc = Cartesian3::default();
            ellipsoid
                .cartographic_to_cartesian(&perimeter_cartographic_nc, &mut perimeter_cartesian_nc);
            let mut perimeter_cartesian_nw = Cartesian3::default();
            ellipsoid
                .cartographic_to_cartesian(&perimeter_cartographic_nw, &mut perimeter_cartesian_nw);
            let mut perimeter_cartesian_cw = Cartesian3::default();
            ellipsoid
                .cartographic_to_cartesian(&perimeter_cartographic_cw, &mut perimeter_cartesian_cw);
            let mut perimeter_cartesian_sw = Cartesian3::default();
            ellipsoid
                .cartographic_to_cartesian(&perimeter_cartographic_sw, &mut perimeter_cartesian_sw);
            let mut perimeter_cartesian_sc = Cartesian3::default();
            ellipsoid
                .cartographic_to_cartesian(&perimeter_cartographic_sc, &mut perimeter_cartesian_sc);

            let perimeter_projected_nc = tangent_plane
                .project_point_to_nearest_tangent_plane(&perimeter_cartesian_nc);
            let perimeter_projected_nw = tangent_plane
                .project_point_to_nearest_tangent_plane(&perimeter_cartesian_nw);
            let perimeter_projected_cw = tangent_plane
                .project_point_to_nearest_tangent_plane(&perimeter_cartesian_cw);
            let perimeter_projected_sw = tangent_plane
                .project_point_to_nearest_tangent_plane(&perimeter_cartesian_sw);
            let perimeter_projected_sc = tangent_plane
                .project_point_to_nearest_tangent_plane(&perimeter_cartesian_sc);

            let min_x = perimeter_projected_nw
                .x
                .min(perimeter_projected_cw.x)
                .min(perimeter_projected_sw.x);
            let max_x = -min_x; // symmetrical

            let max_y = perimeter_projected_nw.y.max(perimeter_projected_nc.y);
            let min_y = perimeter_projected_sw.y.min(perimeter_projected_sc.y);

            // Compute minimum Z using the rectangle at minimum height, since
            // it will be deeper than the maximum height
            let mut perimeter_cartographic_nw = perimeter_cartographic_nw;
            let mut perimeter_cartographic_sw = perimeter_cartographic_sw;
            perimeter_cartographic_nw.height = minimum_height;
            perimeter_cartographic_sw.height = minimum_height;
            let mut perimeter_cartesian_nw = Cartesian3::default();
            ellipsoid
                .cartographic_to_cartesian(&perimeter_cartographic_nw, &mut perimeter_cartesian_nw);
            let mut perimeter_cartesian_sw = Cartesian3::default();
            ellipsoid
                .cartographic_to_cartesian(&perimeter_cartographic_sw, &mut perimeter_cartesian_sw);

            let min_z = Plane::get_point_distance(&plane, &perimeter_cartesian_nw).min(
                Plane::get_point_distance(&plane, &perimeter_cartesian_sw),
            );
            // Since the tangent plane touches the surface at height = 0, this
            // is okay
            let max_z = maximum_height;

            from_plane_extents(
                tangent_plane.origin(),
                tangent_plane.x_axis(),
                tangent_plane.y_axis(),
                &tangent_plane.plane().normal,
                min_x,
                max_x,
                min_y,
                max_y,
                min_z,
                max_z,
                &mut out,
            );
        } else {
            // Handle the case where rectangle width is greater than PI (wraps
            // around more than half the ellipsoid).
            let fully_above_equator = rectangle.south > 0.0;
            let fully_below_equator = rectangle.north < 0.0;
            let latitude_nearest_to_equator = if fully_above_equator {
                rectangle.south
            } else if fully_below_equator {
                rectangle.north
            } else {
                0.0
            };
            let center_longitude = Rectangle::center(rectangle).longitude;

            // Plane is located at the rectangle's center longitude and the
            // rectangle's latitude that is closest to the equator. It rotates
            // around the Z axis.
            let mut plane_origin = Cartesian3::default();
            Cartesian3::from_radians(
                center_longitude,
                latitude_nearest_to_equator,
                Some(maximum_height),
                Some(ellipsoid.radii_squared()),
                &mut plane_origin,
            );
            // center the plane on the equator to simplify plane normal
            // calculation
            plane_origin.z = 0.0;
            let is_pole = plane_origin.x.abs() < CesiumMath::EPSILON10
                && plane_origin.y.abs() < CesiumMath::EPSILON10;
            let mut plane_normal = Cartesian3::UNIT_X;
            if !is_pole {
                Cartesian3::normalize(&plane_origin, &mut plane_normal);
            }
            let plane_y_axis = Cartesian3::UNIT_Z;
            let plane_x_axis = Cartesian3::cross_new(&plane_normal, &plane_y_axis);
            let plane = Plane::from_point_normal_new(&plane_origin, &plane_normal);

            // Get the horizon point relative to the center. This will be the
            // farthest extent in the plane's X dimension.
            let mut horizon_cartesian = Cartesian3::default();
            Cartesian3::from_radians(
                center_longitude + CesiumMath::PI_OVER_TWO,
                latitude_nearest_to_equator,
                Some(maximum_height),
                Some(ellipsoid.radii_squared()),
                &mut horizon_cartesian,
            );
            let horizon_projected =
                Plane::project_point_onto_plane_new(&plane, &horizon_cartesian);
            let max_x = Cartesian3::dot(&horizon_projected, &plane_x_axis);
            let min_x = -max_x; // symmetrical

            // Get the min and max Y, using the height that will give the
            // largest extent
            let max_y = Cartesian3::from_radians_new(
                0.0,
                rectangle.north,
                Some(if fully_below_equator {
                    minimum_height
                } else {
                    maximum_height
                }),
                Some(ellipsoid.radii_squared()),
            )
            .z;
            let min_y = Cartesian3::from_radians_new(
                0.0,
                rectangle.south,
                Some(if fully_above_equator {
                    minimum_height
                } else {
                    maximum_height
                }),
                Some(ellipsoid.radii_squared()),
            )
            .z;

            let mut far_z = Cartesian3::default();
            Cartesian3::from_radians(
                rectangle.east,
                latitude_nearest_to_equator,
                Some(maximum_height),
                Some(ellipsoid.radii_squared()),
                &mut far_z,
            );
            let min_z = Plane::get_point_distance(&plane, &far_z);
            let max_z = 0.0; // plane origin starts at maxZ already

            // min and max are local to the plane axes
            from_plane_extents(
                &plane_origin,
                &plane_x_axis,
                &plane_y_axis,
                &plane_normal,
                min_x,
                max_x,
                min_y,
                max_y,
                min_z,
                max_z,
                &mut out,
            );
        }

        if let Some(result) = result {
            *result = out.clone();
        }
        out
    }

    /// Computes an OrientedBoundingBox that bounds an affine transformation.
    pub fn from_transformation(transformation: &Matrix4, result: Option<&mut Self>) -> Self {
        let mut r = result.map(|r| { *r = Self::default(); r }).cloned()
            .unwrap_or_default();

        r.center = Matrix4::get_translation_new(transformation);
        let half_axes = Matrix4::get_matrix3_new(transformation);
        Matrix3::multiply_by_scalar(&half_axes, 0.5, &mut r.half_axes);
        r
    }

    /// Duplicates an OrientedBoundingBox instance.
    pub fn clone_box(box_: &Self, result: Option<&mut Self>) -> Self {
        match result {
            Some(r) => {
                r.center = box_.center;
                r.half_axes = box_.half_axes;
                r.clone()
            }
            None => box_.clone(),
        }
    }

    /// Determines which side of a plane the oriented bounding box is located.
    pub fn intersect_plane(box_: &Self, plane: &Plane) -> Intersect {
        let center = &box_.center;
        let normal = &plane.normal;
        let half_axes = &box_.half_axes;

        let nx = normal.x;
        let ny = normal.y;
        let nz = normal.z;

        let rad_effective =
            (nx * half_axes.elements[Matrix3::COLUMN0ROW0]
                + ny * half_axes.elements[Matrix3::COLUMN0ROW1]
                + nz * half_axes.elements[Matrix3::COLUMN0ROW2])
                .abs()
                + (nx * half_axes.elements[Matrix3::COLUMN1ROW0]
                    + ny * half_axes.elements[Matrix3::COLUMN1ROW1]
                    + nz * half_axes.elements[Matrix3::COLUMN1ROW2])
                    .abs()
                + (nx * half_axes.elements[Matrix3::COLUMN2ROW0]
                    + ny * half_axes.elements[Matrix3::COLUMN2ROW1]
                    + nz * half_axes.elements[Matrix3::COLUMN2ROW2])
                    .abs();

        let distance_to_plane = Cartesian3::dot(normal, center) + plane.distance;

        if distance_to_plane <= -rad_effective {
            Intersect::Outside
        } else if distance_to_plane >= rad_effective {
            Intersect::Inside
        } else {
            Intersect::Intersecting
        }
    }

    /// Computes the estimated distance squared from the closest point on a bounding box to a point.
    pub fn distance_squared_to(box_: &Self, cartesian: &Cartesian3) -> f64 {
        let mut offset = Cartesian3::ZERO;
        Cartesian3::subtract(cartesian, &box_.center, &mut offset);

        let half_axes = &box_.half_axes;
        let mut u = Matrix3::get_column_new(half_axes, 0);
        let mut v = Matrix3::get_column_new(half_axes, 1);
        let mut w = Matrix3::get_column_new(half_axes, 2);

        let u_half = Cartesian3::magnitude(&u);
        let v_half = Cartesian3::magnitude(&v);
        let w_half = Cartesian3::magnitude(&w);

        let mut u_valid = true;
        let mut v_valid = true;
        let mut w_valid = true;

        if u_half > 0.0 {
            let mut normalized = Cartesian3::ZERO;
            Cartesian3::divide_by_scalar(&u, u_half, &mut normalized);
            u = normalized;
        } else {
            u_valid = false;
        }

        if v_half > 0.0 {
            let mut normalized = Cartesian3::ZERO;
            Cartesian3::divide_by_scalar(&v, v_half, &mut normalized);
            v = normalized;
        } else {
            v_valid = false;
        }

        if w_half > 0.0 {
            let mut normalized = Cartesian3::ZERO;
            Cartesian3::divide_by_scalar(&w, w_half, &mut normalized);
            w = normalized;
        } else {
            w_valid = false;
        }

        let num_degenerate = (!u_valid as i32) + (!v_valid as i32) + (!w_valid as i32);

        if num_degenerate == 1 {
            let mut degenerate_is_u = false;
            let mut degenerate_is_v = false;
            let valid_axis1;
            let valid_axis2;

            if !u_valid {
                valid_axis1 = v;
                valid_axis2 = w;
                degenerate_is_u = true;
            } else if !v_valid {
                valid_axis1 = u;
                valid_axis2 = w;
                degenerate_is_v = true;
            } else {
                valid_axis1 = u;
                valid_axis2 = v;
            }

            let mut valid_axis3 = Cartesian3::ZERO;
            Cartesian3::cross(&valid_axis1, &valid_axis2, &mut valid_axis3);

            if degenerate_is_u {
                u = valid_axis3;
            } else if degenerate_is_v {
                v = valid_axis3;
            } else {
                w = valid_axis3;
            }
        } else if num_degenerate == 2 {
            let valid_axis1 = if u_valid { u } else if v_valid { v } else { w };

            let mut cross_vector = Cartesian3::UNIT_Y;
            if Cartesian3::equals_epsilon_method(
                &cross_vector,
                &valid_axis1,
                Some(CesiumMath::EPSILON3),
                None,
            ) {
                cross_vector = Cartesian3::UNIT_X;
            }

            let mut valid_axis2 = Cartesian3::ZERO;
            Cartesian3::cross(&valid_axis1, &cross_vector, &mut valid_axis2);
            let mut va2_norm = Cartesian3::ZERO;
            Cartesian3::normalize(&valid_axis2, &mut va2_norm);
            valid_axis2 = va2_norm;

            let mut valid_axis3 = Cartesian3::ZERO;
            Cartesian3::cross(&valid_axis1, &valid_axis2, &mut valid_axis3);
            let mut va3_norm = Cartesian3::ZERO;
            Cartesian3::normalize(&valid_axis3, &mut va3_norm);
            valid_axis3 = va3_norm;

            if u_valid {
                v = valid_axis2;
                w = valid_axis3;
            } else if v_valid {
                w = valid_axis2;
                u = valid_axis3;
            } else {
                u = valid_axis2;
                v = valid_axis3;
            }
        } else if num_degenerate == 3 {
            u = Cartesian3::UNIT_X;
            v = Cartesian3::UNIT_Y;
            w = Cartesian3::UNIT_Z;
        }

        let mut p_prime = Cartesian3::ZERO;
        p_prime.x = Cartesian3::dot(&offset, &u);
        p_prime.y = Cartesian3::dot(&offset, &v);
        p_prime.z = Cartesian3::dot(&offset, &w);

        let mut distance_squared = 0.0;

        if p_prime.x < -u_half {
            let d = p_prime.x + u_half;
            distance_squared += d * d;
        } else if p_prime.x > u_half {
            let d = p_prime.x - u_half;
            distance_squared += d * d;
        }

        if p_prime.y < -v_half {
            let d = p_prime.y + v_half;
            distance_squared += d * d;
        } else if p_prime.y > v_half {
            let d = p_prime.y - v_half;
            distance_squared += d * d;
        }

        if p_prime.z < -w_half {
            let d = p_prime.z + w_half;
            distance_squared += d * d;
        } else if p_prime.z > w_half {
            let d = p_prime.z - w_half;
            distance_squared += d * d;
        }

        distance_squared
    }

    /// Computes the distances from a position projected onto a direction to the
    /// nearest and farthest planes of the bounding box.
    pub fn compute_plane_distances(
        box_: &Self,
        position: &Cartesian3,
        direction: &Cartesian3,
        result: Option<&mut Interval>,
    ) -> Interval {
        let mut r = result.cloned().unwrap_or(Interval { start: 0.0, stop: 0.0 });

        let mut min_dist = f64::INFINITY;
        let mut max_dist = f64::NEG_INFINITY;

        let center = &box_.center;
        let half_axes = &box_.half_axes;

        let u = Matrix3::get_column_new(half_axes, 0);
        let v = Matrix3::get_column_new(half_axes, 1);
        let w = Matrix3::get_column_new(half_axes, 2);

        // Helper: project a corner onto direction
        let project = |corner: &Cartesian3| -> f64 {
            let mut to_center = Cartesian3::ZERO;
            Cartesian3::subtract(corner, position, &mut to_center);
            Cartesian3::dot(direction, &to_center)
        };

        // 8 corners: ±u ±v ±w + center
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

        for (su, sv, sw) in &signs {
            let corner = *center;
            let mut tmp = Cartesian3::ZERO;
            Cartesian3::multiply_by_scalar(&u, *su, &mut tmp);
            let mut c = Cartesian3::ZERO;
            Cartesian3::add(&corner, &tmp, &mut c);
            Cartesian3::multiply_by_scalar(&v, *sv, &mut tmp);
            let mut c2 = Cartesian3::ZERO;
            Cartesian3::add(&c, &tmp, &mut c2);
            Cartesian3::multiply_by_scalar(&w, *sw, &mut tmp);
            let mut c3 = Cartesian3::ZERO;
            Cartesian3::add(&c2, &tmp, &mut c3);

            let mag = project(&c3);
            min_dist = min_dist.min(mag);
            max_dist = max_dist.max(mag);
        }

        r.start = min_dist;
        r.stop = max_dist;
        r
    }

    /// Computes the eight corners of an oriented bounding box.
    /// Order: (-X,-Y,-Z), (-X,-Y,+Z), (-X,+Y,-Z), (-X,+Y,+Z),
    ///        (+X,-Y,-Z), (+X,-Y,+Z), (+X,+Y,-Z), (+X,+Y,+Z).
    pub fn compute_corners(box_: &Self, result: Option<&mut [Cartesian3; 8]>) -> [Cartesian3; 8] {
        let mut corners = result.copied().unwrap_or([Cartesian3::ZERO; 8]);

        let center = &box_.center;
        let half_axes = &box_.half_axes;
        let x_axis = Matrix3::get_column_new(half_axes, 0);
        let y_axis = Matrix3::get_column_new(half_axes, 1);
        let z_axis = Matrix3::get_column_new(half_axes, 2);

        let signs: [(f64, f64, f64); 8] = [
            (-1.0, -1.0, -1.0),
            (-1.0, -1.0, 1.0),
            (-1.0, 1.0, -1.0),
            (-1.0, 1.0, 1.0),
            (1.0, -1.0, -1.0),
            (1.0, -1.0, 1.0),
            (1.0, 1.0, -1.0),
            (1.0, 1.0, 1.0),
        ];

        for (i, (sx, sy, sz)) in signs.iter().enumerate() {
            let mut tmp_x = Cartesian3::ZERO;
            Cartesian3::multiply_by_scalar(&x_axis, *sx, &mut tmp_x);
            let mut tmp_y = Cartesian3::ZERO;
            Cartesian3::multiply_by_scalar(&y_axis, *sy, &mut tmp_y);
            let mut tmp_z = Cartesian3::ZERO;
            Cartesian3::multiply_by_scalar(&z_axis, *sz, &mut tmp_z);

            let mut c = Cartesian3::ZERO;
            Cartesian3::add(center, &tmp_x, &mut c);
            let mut c2 = Cartesian3::ZERO;
            Cartesian3::add(&c, &tmp_y, &mut c2);
            Cartesian3::add(&c2, &tmp_z, &mut corners[i]);
        }

        corners
    }

    /// Computes a transformation matrix from an oriented bounding box.
    pub fn compute_transformation(box_: &Self, result: &mut Matrix4) {
        let mut rotation_scale = Matrix3::ZERO;
        Matrix3::multiply_by_uniform_scale(&box_.half_axes, 2.0, &mut rotation_scale);
        Matrix4::from_rotation_translation(&rotation_scale, &box_.center, result);
    }

    /// Compares two OrientedBoundingBoxes componentwise.
    pub fn equals_static(left: Option<&Self>, right: Option<&Self>) -> bool {
        match (left, right) {
            (Some(l), Some(r)) => {
                Cartesian3::equals(Some(&l.center), Some(&r.center))
                    && Matrix3::equals(&l.half_axes, &r.half_axes)
            }
            (None, None) => true,
            _ => false,
        }
    }

    /// Instance method: determines which side of a plane the box is located.
    pub fn intersect_plane_instance(&self, plane: &Plane) -> Intersect {
        Self::intersect_plane(self, plane)
    }

    /// Instance method: distance squared to a point.
    pub fn distance_squared_to_instance(&self, cartesian: &Cartesian3) -> f64 {
        Self::distance_squared_to(self, cartesian)
    }

    /// Instance method: compute plane distances.
    pub fn compute_plane_distances_instance(
        &self,
        position: &Cartesian3,
        direction: &Cartesian3,
        result: Option<&mut Interval>,
    ) -> Interval {
        Self::compute_plane_distances(self, position, direction, result)
    }

    /// Instance method: compute corners.
    pub fn compute_corners_instance(&self, result: Option<&mut [Cartesian3; 8]>) -> [Cartesian3; 8] {
        Self::compute_corners(self, result)
    }

    /// Instance method: compute transformation.
    pub fn compute_transformation_instance(&self, result: &mut Matrix4) {
        Self::compute_transformation(self, result);
    }

    /// Instance equals.
    pub fn equals(&self, right: Option<&Self>) -> bool {
        Self::equals_static(Some(self), right)
    }
}

/// Mirrors the private `fromPlaneExtents` function.
///
/// DEVIATION: the JS `defined(...)` debug checks on the six extents are
/// compile-time guaranteed in Rust (`f64` parameters).
#[allow(clippy::too_many_arguments)]
fn from_plane_extents(
    plane_origin: &Cartesian3,
    plane_x_axis: &Cartesian3,
    plane_y_axis: &Cartesian3,
    plane_z_axis: &Cartesian3,
    minimum_x: f64,
    maximum_x: f64,
    minimum_y: f64,
    maximum_y: f64,
    minimum_z: f64,
    maximum_z: f64,
    result: &mut OrientedBoundingBox,
) {
    // JS aliases halfAxes as both source and destination of setColumn; the
    // Rust port steps through temporaries to keep the borrows disjoint.
    let half_axes = result.half_axes;
    let mut step = Matrix3::default();
    Matrix3::set_column(&half_axes, 0, plane_x_axis, &mut step);
    let mut half_axes = Matrix3::default();
    Matrix3::set_column(&step, 1, plane_y_axis, &mut half_axes);
    Matrix3::set_column(&half_axes, 2, plane_z_axis, &mut step);

    let center_offset = Cartesian3::new(
        (minimum_x + maximum_x) / 2.0,
        (minimum_y + maximum_y) / 2.0,
        (minimum_z + maximum_z) / 2.0,
    );

    let scale = Cartesian3::new(
        (maximum_x - minimum_x) / 2.0,
        (maximum_y - minimum_y) / 2.0,
        (maximum_z - minimum_z) / 2.0,
    );

    let mut center_offset = center_offset;
    let mut transformed_offset = Cartesian3::default();
    Matrix3::multiply_by_vector(&step, &center_offset, &mut transformed_offset);
    center_offset = transformed_offset;
    Cartesian3::add(plane_origin, &center_offset, &mut result.center);
    Matrix3::multiply_by_scale(&step, &scale, &mut result.half_axes);
}
