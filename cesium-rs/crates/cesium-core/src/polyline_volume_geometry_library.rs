//! Ported from `packages/engine/Source/Core/PolylineVolumeGeometryLibrary.js`.
//!
//! Internal helpers shared by `PolylineVolumeGeometry` and
//! `CorridorGeometryLibrary`: cross-section placement along a subdivided
//! centerline, corner handling, and shape conversion utilities.
//!
//! DEVIATION: JS `computePositions` reads `geometry._ellipsoid`,
//! `geometry._granularity` and `geometry._cornerType`; the Rust port takes a
//! [`ComputePositionsGeometry`] struct with those fields instead.
//!
//! DEVIATION: JS `EllipsoidTangentPlane.projectPointOntoPlane` delegates to
//! `projectPointToNearestTangentPlane`; the Rust port calls the latter
//! directly since no separate wrapper exists.
//!
//! DEVIATION: the module-level `scaleMatrix` (identity with `elements[0]`
//! replaced by `xScalar`) is folded into a scalar multiply of the `x`
//! component, which is numerically identical.

use crate::bounding_rectangle::BoundingRectangle;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::corner_type::CornerType;
use crate::developer_error::throw_developer_error;
use crate::ellipsoid::Ellipsoid;
use crate::ellipsoid_tangent_plane::EllipsoidTangentPlane;
use crate::math::CesiumMath;
use crate::matrix3::Matrix3;
use crate::matrix4::Matrix4;
use crate::one_time_warning::one_time_warning;
use crate::polyline_pipeline::{GenerateArcOptions, PolylinePipeline};
use crate::quaternion::Quaternion;
use crate::transforms::east_north_up_to_fixed_frame;

/// Computes polyline volume geometry.
pub struct PolylineVolumeGeometryLibrary {
    _private: (),
}

/// Rust stand-in for the `PolylineVolumeGeometry` internals accessed by
/// `computePositions` (JS `geometry._ellipsoid`, `geometry._granularity`,
/// `geometry._cornerType`).
#[derive(Clone, Debug)]
pub struct ComputePositionsGeometry {
    pub ellipsoid: Ellipsoid,
    pub granularity: f64,
    pub corner_type: CornerType,
}

fn scale_to_surface(positions: &mut [Cartesian3], ellipsoid: &Ellipsoid) -> Vec<f64> {
    let mut heights = vec![0.0; positions.len()];
    for i in 0..positions.len() {
        let pos = positions[i];
        let mut cartographic = Cartographic::default();
        ellipsoid.cartesian_to_cartographic(&pos, &mut cartographic);
        heights[i] = cartographic.height;
        let mut scaled = Cartesian3::default();
        ellipsoid.scale_to_geodetic_surface(&pos, &mut scaled);
        positions[i] = scaled;
    }
    heights
}

fn subdivide_heights(points: &[Cartesian3], h0: f64, h1: f64, granularity: f64) -> Vec<f64> {
    let p0 = points[0];
    let p1 = points[1];
    let angle_between = Cartesian3::angle_between(&p0, &p1);
    let num_points = (angle_between / granularity).ceil() as usize;
    let mut heights = vec![0.0; num_points];
    if h0 == h1 {
        for i in 0..num_points {
            heights[i] = h0;
        }
        heights.push(h1);
        return heights;
    }

    let d_height = h1 - h0;
    let height_per_vertex = d_height / num_points as f64;

    for i in 1..num_points {
        heights[i] = h0 + i as f64 * height_per_vertex;
    }

    heights[0] = h0;
    heights.push(h1);
    heights
}

fn compute_rotation_angle(
    start: &Cartesian3,
    end: &Cartesian3,
    position: &Cartesian3,
    ellipsoid: &Ellipsoid,
) -> f64 {
    let tangent_plane = match EllipsoidTangentPlane::new(position, Some(ellipsoid.clone())) {
        Some(tp) => tp,
        None => throw_developer_error(
            "tangent plane origin must not be at the center of the ellipsoid",
        ),
    };
    let next = tangent_plane
        .project_point_to_nearest_tangent_plane(&Cartesian3::add_new(position, start));
    let prev = tangent_plane
        .project_point_to_nearest_tangent_plane(&Cartesian3::add_new(position, end));
    let angle = Cartesian2::angle_between(&next, &prev);

    if prev.x * next.y - prev.y * next.x >= 0.0 {
        -angle
    } else {
        angle
    }
}

const NEGATIVE_X: Cartesian3 = Cartesian3 {
    x: -1.0,
    y: 0.0,
    z: 0.0,
};

#[allow(clippy::too_many_arguments)]
fn add_position(
    center: &Cartesian3,
    left: &Cartesian3,
    shape: &[f64],
    final_positions: &mut Vec<f64>,
    ellipsoid: &Ellipsoid,
    height: f64,
    x_scalar: f64,
    repeat: usize,
) {
    let mut transform = Matrix4::default();
    if !east_north_up_to_fixed_frame(center, Some(ellipsoid), &mut transform) {
        if cfg!(debug_assertions) {
            throw_developer_error(
                "east_north_up_to_fixed_frame failed: center is at the ellipsoid center",
            );
        }
    }

    let mut west = Cartesian3::default();
    Matrix4::multiply_by_point_as_vector(&transform, &NEGATIVE_X, &mut west);
    west = Cartesian3::normalize_new(&west);
    let angle = compute_rotation_angle(&west, left, center, ellipsoid);
    let rotation_z = Matrix3::from_rotation_z_new(angle);

    let height_cartesian = Cartesian3 {
        x: 0.0,
        y: 0.0,
        z: height,
    };
    let rotation_translation = Matrix4::from_rotation_translation_new(&rotation_z, &height_cartesian);
    // DEVIATION: JS does `transform = multiplyTransformation(transform, ..., transform)`
    // in place; Rust needs a distinct temporary to avoid simultaneous borrows.
    transform = Matrix4::multiply_transformation_new(&transform, &rotation_translation);

    for _ in 0..repeat {
        let mut i = 0;
        while i < shape.len() {
            // DEVIATION: the JS scale matrix is identity with elements[0] =
            // xScalar, i.e. it scales only the x component.
            let final_position = Cartesian3 {
                x: shape[i] * x_scalar,
                y: shape[i + 1],
                z: shape[i + 2],
            };
            let mut transformed = Cartesian3::default();
            Matrix4::multiply_by_point(&transform, &final_position, &mut transformed);
            final_positions.push(transformed.x);
            final_positions.push(transformed.y);
            final_positions.push(transformed.z);
            i += 3;
        }
    }
}

fn add_positions(
    centers: &[f64],
    left: &Cartesian3,
    shape: &[f64],
    final_positions: &mut Vec<f64>,
    ellipsoid: &Ellipsoid,
    heights: &[f64],
    x_scalar: f64,
) {
    let mut i = 0;
    while i < centers.len() {
        let center = Cartesian3 {
            x: centers[i],
            y: centers[i + 1],
            z: centers[i + 2],
        };
        add_position(
            &center,
            left,
            shape,
            final_positions,
            ellipsoid,
            heights[i / 3],
            x_scalar,
            1,
        );
        i += 3;
    }
}

fn convert_shape_to_3d_duplicate(
    shape2d: &[Cartesian2],
    bounding_rectangle: &BoundingRectangle,
) -> Vec<f64> {
    // orientate 2D shape to XZ plane center at (0, 0, 0), duplicate points
    let length = shape2d.len();
    let mut shape = vec![0.0; length * 6];
    let mut index = 0;
    let x_offset = bounding_rectangle.x + bounding_rectangle.width / 2.0;
    let y_offset = bounding_rectangle.y + bounding_rectangle.height / 2.0;

    let mut point = shape2d[0];
    shape[index] = point.x - x_offset;
    index += 1;
    shape[index] = 0.0;
    index += 1;
    shape[index] = point.y - y_offset;
    index += 1;
    for i in 1..length {
        point = shape2d[i];
        let x = point.x - x_offset;
        let z = point.y - y_offset;
        shape[index] = x;
        index += 1;
        shape[index] = 0.0;
        index += 1;
        shape[index] = z;
        index += 1;

        shape[index] = x;
        index += 1;
        shape[index] = 0.0;
        index += 1;
        shape[index] = z;
        index += 1;
    }
    point = shape2d[0];
    shape[index] = point.x - x_offset;
    index += 1;
    shape[index] = 0.0;
    index += 1;
    shape[index] = point.y - y_offset;

    shape
}

fn convert_shape_to_3d(shape2d: &[Cartesian2], bounding_rectangle: &BoundingRectangle) -> Vec<f64> {
    // orientate 2D shape to XZ plane center at (0, 0, 0)
    let length = shape2d.len();
    let mut shape = vec![0.0; length * 3];
    let mut index = 0;
    let x_offset = bounding_rectangle.x + bounding_rectangle.width / 2.0;
    let y_offset = bounding_rectangle.y + bounding_rectangle.height / 2.0;

    for i in 0..length {
        shape[index] = shape2d[i].x - x_offset;
        index += 1;
        shape[index] = 0.0;
        index += 1;
        shape[index] = shape2d[i].y - y_offset;
        index += 1;
    }

    shape
}

#[allow(clippy::too_many_arguments)]
fn compute_round_corner(
    pivot: &Cartesian3,
    start_point: &Cartesian3,
    end_point: &Cartesian3,
    corner_type: CornerType,
    left_is_outside: bool,
    ellipsoid: &Ellipsoid,
    final_positions: &mut Vec<f64>,
    shape: &[f64],
    height: f64,
    duplicate_points: bool,
) {
    let angle = Cartesian3::angle_between(
        &Cartesian3::subtract_new(start_point, pivot),
        &Cartesian3::subtract_new(end_point, pivot),
    );
    let granularity = if corner_type == CornerType::Beveled {
        0
    } else {
        (angle / CesiumMath::to_radians(5.0)).ceil() as usize
    };

    let axis = if left_is_outside {
        Cartesian3::negate_new(pivot)
    } else {
        *pivot
    };
    let quaternion = Quaternion::from_axis_angle_new(&axis, angle / (granularity + 1) as f64);
    let mut m = Matrix3::default();
    Matrix3::from_quaternion(&quaternion, &mut m);

    let mut start_point = *start_point;
    if granularity > 0 {
        let repeat = if duplicate_points { 2 } else { 1 };
        for _ in 0..granularity {
            let mut rotated = Cartesian3::default();
            Matrix3::multiply_by_vector(&m, &start_point, &mut rotated);
            start_point = rotated;
            let mut left = Cartesian3::subtract_new(&start_point, pivot);
            left = Cartesian3::normalize_new(&left);
            if !left_is_outside {
                left = Cartesian3::negate_new(&left);
            }
            let mut surface_point = Cartesian3::default();
            ellipsoid.scale_to_geodetic_surface(&start_point, &mut surface_point);
            add_position(
                &surface_point,
                &left,
                shape,
                final_positions,
                ellipsoid,
                height,
                1.0,
                repeat,
            );
        }
    } else {
        let mut left = Cartesian3::subtract_new(&start_point, pivot);
        left = Cartesian3::normalize_new(&left);
        if !left_is_outside {
            left = Cartesian3::negate_new(&left);
        }
        let mut surface_point = Cartesian3::default();
        ellipsoid.scale_to_geodetic_surface(&start_point, &mut surface_point);
        add_position(
            &surface_point,
            &left,
            shape,
            final_positions,
            ellipsoid,
            height,
            1.0,
            1,
        );

        let end_point = *end_point;
        let mut left = Cartesian3::subtract_new(&end_point, pivot);
        left = Cartesian3::normalize_new(&left);
        if !left_is_outside {
            left = Cartesian3::negate_new(&left);
        }
        let mut surface_point = Cartesian3::default();
        ellipsoid.scale_to_geodetic_surface(&end_point, &mut surface_point);
        add_position(
            &surface_point,
            &left,
            shape,
            final_positions,
            ellipsoid,
            height,
            1.0,
            1,
        );
    }
}

impl PolylineVolumeGeometryLibrary {
    /// Creates a new PolylineVolumeGeometryLibrary.
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Port of `PolylineVolumeGeometryLibrary.removeDuplicatesFromShape`.
    pub fn remove_duplicates_from_shape(shape_positions: &[Cartesian2]) -> Vec<Cartesian2> {
        let length = shape_positions.len();
        let mut cleaned_positions: Vec<Cartesian2> = Vec::new();
        if length == 0 {
            return cleaned_positions;
        }
        let mut i0 = length - 1;
        for i1 in 0..length {
            let v0 = &shape_positions[i0];
            let v1 = &shape_positions[i1];

            if !Cartesian2::equals(Some(v0), Some(v1)) {
                cleaned_positions.push(*v1); // Shallow copy!
            }
            i0 = i1;
        }

        cleaned_positions
    }

    /// Port of `PolylineVolumeGeometryLibrary.angleIsGreaterThanPi`.
    pub fn angle_is_greater_than_pi(
        forward: &Cartesian3,
        backward: &Cartesian3,
        position: &Cartesian3,
        ellipsoid: &Ellipsoid,
    ) -> bool {
        let tangent_plane = match EllipsoidTangentPlane::new(position, Some(ellipsoid.clone())) {
            Some(tp) => tp,
            None => throw_developer_error(
                "tangent plane origin must not be at the center of the ellipsoid",
            ),
        };
        let next = tangent_plane
            .project_point_to_nearest_tangent_plane(&Cartesian3::add_new(position, forward));
        let prev = tangent_plane
            .project_point_to_nearest_tangent_plane(&Cartesian3::add_new(position, backward));

        prev.x * next.y - prev.y * next.x >= 0.0
    }

    /// Port of `PolylineVolumeGeometryLibrary.computePositions`.
    ///
    /// DEVIATION: JS mutates the caller's `positions` array via
    /// `scaleToSurface`; the Rust port takes `&mut [Cartesian3]` explicitly.
    pub fn compute_positions(
        positions: &mut [Cartesian3],
        shape2d: &[Cartesian2],
        bounding_rectangle: &BoundingRectangle,
        geometry: &ComputePositionsGeometry,
        duplicate_points: bool,
    ) -> Vec<f64> {
        let ellipsoid = &geometry.ellipsoid;
        let heights = scale_to_surface(positions, ellipsoid);
        let granularity = geometry.granularity;
        let corner_type = geometry.corner_type;
        let shape_for_sides = if duplicate_points {
            convert_shape_to_3d_duplicate(shape2d, bounding_rectangle)
        } else {
            convert_shape_to_3d(shape2d, bounding_rectangle)
        };
        let shape_for_ends: Option<Vec<f64>> = if duplicate_points {
            Some(convert_shape_to_3d(shape2d, bounding_rectangle))
        } else {
            None
        };
        let height_offset = bounding_rectangle.height / 2.0;
        let width = bounding_rectangle.width / 2.0;
        let length = positions.len();
        let mut final_positions: Vec<f64> = Vec::new();
        let mut ends: Vec<f64> = Vec::new();

        let position = positions[0];
        let mut next_position = positions[1];
        let mut surface_normal = Cartesian3::default();
        ellipsoid.geodetic_surface_normal(&position, &mut surface_normal);
        let mut forward = Cartesian3::subtract_new(&next_position, &position);
        forward = Cartesian3::normalize_new(&forward);
        let mut left = Cartesian3::cross_new(&surface_normal, &forward);
        left = Cartesian3::normalize_new(&left);
        let mut h0 = heights[0];
        let mut h1 = heights[1];
        if duplicate_points {
            add_position(
                &position,
                &left,
                shape_for_ends.as_ref().unwrap(),
                &mut ends,
                ellipsoid,
                h0 + height_offset,
                1.0,
                1,
            );
        }
        let mut previous_position = position;
        let mut position = next_position;
        let mut backward = Cartesian3::negate_new(&forward);
        for i in 1..length - 1 {
            let repeat = if duplicate_points { 2 } else { 1 };
            next_position = positions[i + 1];
            if Cartesian3::equals(Some(&position), Some(&next_position)) {
                one_time_warning(
                    Some(
                        "Positions are too close and are considered equivalent with rounding error.",
                    ),
                    None,
                );
                continue;
            }
            forward = Cartesian3::normalize_new(&Cartesian3::subtract_new(
                &next_position,
                &position,
            ));
            ellipsoid.geodetic_surface_normal(&position, &mut surface_normal);

            let mut forward_projection = Cartesian3::multiply_by_scalar_new(
                &surface_normal,
                Cartesian3::dot(&forward, &surface_normal),
            );
            forward_projection = Cartesian3::subtract_new(&forward, &forward_projection);
            forward_projection = Cartesian3::normalize_new(&forward_projection);

            let mut backward_projection = Cartesian3::multiply_by_scalar_new(
                &surface_normal,
                Cartesian3::dot(&backward, &surface_normal),
            );
            backward_projection = Cartesian3::subtract_new(&backward, &backward_projection);
            backward_projection = Cartesian3::normalize_new(&backward_projection);

            let do_corner = !CesiumMath::equals_epsilon(
                Cartesian3::dot(&forward_projection, &backward_projection).abs(),
                1.0,
                Some(CesiumMath::EPSILON7),
                None,
            );

            if do_corner {
                let mut corner_direction = Cartesian3::add_new(&forward, &backward);
                corner_direction = Cartesian3::normalize_new(&corner_direction);
                corner_direction = Cartesian3::cross_new(&corner_direction, &surface_normal);
                corner_direction = Cartesian3::cross_new(&surface_normal, &corner_direction);
                corner_direction = Cartesian3::normalize_new(&corner_direction);
                let scalar = 1.0
                    / 0.25_f64.max(Cartesian3::magnitude(&Cartesian3::cross_new(
                        &corner_direction,
                        &backward,
                    )));
                let left_is_outside = PolylineVolumeGeometryLibrary::angle_is_greater_than_pi(
                    &forward, &backward, &position, ellipsoid,
                );
                if left_is_outside {
                    let pivot = Cartesian3::add_new(
                        &position,
                        &Cartesian3::multiply_by_scalar_new(&corner_direction, scalar * width),
                    );
                    let start = Cartesian3::add_new(
                        &pivot,
                        &Cartesian3::multiply_by_scalar_new(&left, width),
                    );
                    let subdivided_heights = subdivide_heights(
                        &[previous_position, start],
                        h0 + height_offset,
                        h1 + height_offset,
                        granularity,
                    );
                    let subdivided_positions = PolylinePipeline::generate_arc(Some(&GenerateArcOptions {
                        positions: vec![previous_position, start],
                        granularity: Some(granularity),
                        ellipsoid: Some(ellipsoid.clone()),
                        ..Default::default()
                    }));
                    add_positions(
                        &subdivided_positions,
                        &left,
                        &shape_for_sides,
                        &mut final_positions,
                        ellipsoid,
                        &subdivided_heights,
                        1.0,
                    );
                    left = Cartesian3::normalize_new(&Cartesian3::cross_new(
                        &surface_normal,
                        &forward,
                    ));
                    let end = Cartesian3::add_new(
                        &pivot,
                        &Cartesian3::multiply_by_scalar_new(&left, width),
                    );
                    if corner_type == CornerType::Rounded || corner_type == CornerType::Beveled {
                        compute_round_corner(
                            &pivot,
                            &start,
                            &end,
                            corner_type,
                            left_is_outside,
                            ellipsoid,
                            &mut final_positions,
                            &shape_for_sides,
                            h1 + height_offset,
                            duplicate_points,
                        );
                    } else {
                        let negated_corner_direction = Cartesian3::negate_new(&corner_direction);
                        add_position(
                            &position,
                            &negated_corner_direction,
                            &shape_for_sides,
                            &mut final_positions,
                            ellipsoid,
                            h1 + height_offset,
                            scalar,
                            repeat,
                        );
                    }
                    previous_position = end;
                } else {
                    let pivot = Cartesian3::add_new(
                        &position,
                        &Cartesian3::multiply_by_scalar_new(&corner_direction, scalar * width),
                    );
                    let start = Cartesian3::add_new(
                        &pivot,
                        &Cartesian3::multiply_by_scalar_new(&left, -width),
                    );
                    let subdivided_heights = subdivide_heights(
                        &[previous_position, start],
                        h0 + height_offset,
                        h1 + height_offset,
                        granularity,
                    );
                    let subdivided_positions = PolylinePipeline::generate_arc(Some(&GenerateArcOptions {
                        positions: vec![previous_position, start],
                        granularity: Some(granularity),
                        ellipsoid: Some(ellipsoid.clone()),
                        ..Default::default()
                    }));
                    add_positions(
                        &subdivided_positions,
                        &left,
                        &shape_for_sides,
                        &mut final_positions,
                        ellipsoid,
                        &subdivided_heights,
                        1.0,
                    );
                    left = Cartesian3::normalize_new(&Cartesian3::cross_new(
                        &surface_normal,
                        &forward,
                    ));
                    let end = Cartesian3::add_new(
                        &pivot,
                        &Cartesian3::multiply_by_scalar_new(&left, -width),
                    );
                    if corner_type == CornerType::Rounded || corner_type == CornerType::Beveled {
                        compute_round_corner(
                            &pivot,
                            &start,
                            &end,
                            corner_type,
                            left_is_outside,
                            ellipsoid,
                            &mut final_positions,
                            &shape_for_sides,
                            h1 + height_offset,
                            duplicate_points,
                        );
                    } else {
                        add_position(
                            &position,
                            &corner_direction,
                            &shape_for_sides,
                            &mut final_positions,
                            ellipsoid,
                            h1 + height_offset,
                            scalar,
                            repeat,
                        );
                    }
                    previous_position = end;
                }
                backward = Cartesian3::negate_new(&forward);
            } else {
                add_position(
                    &previous_position,
                    &left,
                    &shape_for_sides,
                    &mut final_positions,
                    ellipsoid,
                    h0 + height_offset,
                    1.0,
                    1,
                );
                previous_position = position;
            }
            h0 = h1;
            h1 = heights[i + 1];
            position = next_position;
        }

        let subdivided_heights = subdivide_heights(
            &[previous_position, position],
            h0 + height_offset,
            h1 + height_offset,
            granularity,
        );
        let subdivided_positions = PolylinePipeline::generate_arc(Some(&GenerateArcOptions {
            positions: vec![previous_position, position],
            granularity: Some(granularity),
            ellipsoid: Some(ellipsoid.clone()),
            ..Default::default()
        }));
        add_positions(
            &subdivided_positions,
            &left,
            &shape_for_sides,
            &mut final_positions,
            ellipsoid,
            &subdivided_heights,
            1.0,
        );
        if duplicate_points {
            add_position(
                &position,
                &left,
                shape_for_ends.as_ref().unwrap(),
                &mut ends,
                ellipsoid,
                h1 + height_offset,
                1.0,
                1,
            );
        }

        if duplicate_points {
            final_positions.extend_from_slice(&ends);
        }

        final_positions
    }
}

impl Default for PolylineVolumeGeometryLibrary {
    fn default() -> Self {
        Self::new()
    }
}
