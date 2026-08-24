//! Ported from `packages/engine/Source/Core/CorridorGeometryLibrary.js`.
//!
//! Internal helpers for `CorridorGeometry`: centerline offset computation with
//! corner handling, end caps, and attribute insertion.
//!
//! DEVIATION: JS `computePositions` takes a free-form `params` object; the
//! Rust port uses [`CorridorComputePositionsParams`].
//!
//! DEVIATION: JS corner entries are `{ leftPositions }` / `{ rightPositions }`
//! objects; the Rust port uses the [`CorridorCorner`] enum.

use crate::cartesian3::Cartesian3;
use crate::corner_type::CornerType;
use crate::ellipsoid::Ellipsoid;
use crate::math::CesiumMath;
use crate::matrix3::Matrix3;
use crate::polyline_pipeline::{GenerateArcOptions, PolylinePipeline};
use crate::polyline_volume_geometry_library::PolylineVolumeGeometryLibrary;
use crate::quaternion::Quaternion;

/// Corridor geometry helpers.
pub struct CorridorGeometryLibrary {
    _private: (),
}

/// Params for [`CorridorGeometryLibrary::compute_positions`] (mirrors the JS
/// `params` object).
#[derive(Clone, Debug)]
pub struct CorridorComputePositionsParams {
    pub granularity: f64,
    pub positions: Vec<Cartesian3>,
    pub ellipsoid: Ellipsoid,
    pub width: f64,
    pub corner_type: CornerType,
    pub save_attributes: bool,
}

/// A corner produced by [`CorridorGeometryLibrary::compute_positions`]:
/// JS `{ leftPositions }` or `{ rightPositions }`.
#[derive(Clone, Debug)]
pub enum CorridorCorner {
    LeftPositions(Vec<f64>),
    RightPositions(Vec<f64>),
}

/// Result of [`CorridorGeometryLibrary::compute_positions`].
#[derive(Clone, Debug)]
pub struct CorridorComputePositionsResult {
    pub positions: Vec<Vec<f64>>,
    pub corners: Vec<CorridorCorner>,
    pub lefts: Option<Vec<f64>>,
    pub normals: Option<Vec<f64>>,
    pub end_positions: Option<Vec<Vec<f64>>>,
}

fn compute_round_corner(
    corner_point: &Cartesian3,
    start_point: &Cartesian3,
    end_point: &Cartesian3,
    corner_type: CornerType,
    left_is_outside: bool,
) -> Vec<f64> {
    let angle = Cartesian3::angle_between(
        &Cartesian3::subtract_new(start_point, corner_point),
        &Cartesian3::subtract_new(end_point, corner_point),
    );
    let granularity = if corner_type == CornerType::Beveled {
        1
    } else {
        (angle / CesiumMath::to_radians(5.0)).ceil() as usize + 1
    };

    let size = granularity * 3;
    let mut array = vec![0.0; size];

    array[size - 3] = end_point.x;
    array[size - 2] = end_point.y;
    array[size - 1] = end_point.z;

    let axis = if left_is_outside {
        Cartesian3::negate_new(corner_point)
    } else {
        *corner_point
    };
    let quaternion = Quaternion::from_axis_angle_new(&axis, angle / granularity as f64);
    let mut m = Matrix3::default();
    Matrix3::from_quaternion(&quaternion, &mut m);

    let mut index = 0;
    let mut start_point = *start_point;
    for _ in 0..granularity {
        let mut rotated = Cartesian3::default();
        Matrix3::multiply_by_vector(&m, &start_point, &mut rotated);
        start_point = rotated;
        array[index] = start_point.x;
        index += 1;
        array[index] = start_point.y;
        index += 1;
        array[index] = start_point.z;
        index += 1;
    }

    array
}

fn add_end_caps(calculated_positions: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let left_edge = &calculated_positions[1];
    let mut start_point = Cartesian3::default();
    Cartesian3::from_array(left_edge, Some(left_edge.len() - 3), &mut start_point);
    let mut end_point = Cartesian3::default();
    Cartesian3::from_array(&calculated_positions[0], Some(0), &mut end_point);
    let mut corner_point = Cartesian3::default();
    Cartesian3::midpoint(&start_point, &end_point, &mut corner_point);
    let first_end_cap = compute_round_corner(
        &corner_point,
        &start_point,
        &end_point,
        CornerType::Rounded,
        false,
    );

    let length = calculated_positions.len() - 1;
    let right_edge = &calculated_positions[length - 1];
    let left_edge = &calculated_positions[length];
    let mut start_point = Cartesian3::default();
    Cartesian3::from_array(right_edge, Some(right_edge.len() - 3), &mut start_point);
    let mut end_point = Cartesian3::default();
    Cartesian3::from_array(left_edge, Some(0), &mut end_point);
    let mut corner_point = Cartesian3::default();
    Cartesian3::midpoint(&start_point, &end_point, &mut corner_point);
    let last_end_cap = compute_round_corner(
        &corner_point,
        &start_point,
        &end_point,
        CornerType::Rounded,
        false,
    );

    vec![first_end_cap, last_end_cap]
}

fn compute_mitered_corner(
    position: &Cartesian3,
    left_corner_direction: &Cartesian3,
    last_point: &Cartesian3,
    left_is_outside: bool,
) -> Vec<f64> {
    let corner_point = if left_is_outside {
        Cartesian3::add_new(position, left_corner_direction)
    } else {
        let negated = Cartesian3::negate_new(left_corner_direction);
        Cartesian3::add_new(position, &negated)
    };
    vec![
        corner_point.x,
        corner_point.y,
        corner_point.z,
        last_point.x,
        last_point.y,
        last_point.z,
    ]
}

fn add_shifted_positions(
    positions: &[f64],
    left: &Cartesian3,
    scalar: f64,
    calculated_positions: &mut Vec<Vec<f64>>,
) {
    let mut right_positions = vec![0.0; positions.len()];
    let mut left_positions = vec![0.0; positions.len()];
    let scaled_left = Cartesian3::multiply_by_scalar_new(left, scalar);
    let scaled_right = Cartesian3::negate_new(&scaled_left);
    let mut right_index = 0;
    // DEVIATION: JS lets `leftIndex` go negative after the final write; the
    // Rust port uses `isize` to mirror that without underflowing.
    let mut left_index = positions.len() as isize - 1;

    let mut i = 0;
    while i < positions.len() {
        let mut pos = Cartesian3::default();
        Cartesian3::from_array(positions, Some(i), &mut pos);
        let right_pos = Cartesian3::add_new(&pos, &scaled_right);
        right_positions[right_index] = right_pos.x;
        right_index += 1;
        right_positions[right_index] = right_pos.y;
        right_index += 1;
        right_positions[right_index] = right_pos.z;
        right_index += 1;

        let left_pos = Cartesian3::add_new(&pos, &scaled_left);
        left_positions[left_index as usize] = left_pos.z;
        left_index -= 1;
        left_positions[left_index as usize] = left_pos.y;
        left_index -= 1;
        left_positions[left_index as usize] = left_pos.x;
        left_index -= 1;
        i += 3;
    }
    calculated_positions.push(right_positions);
    calculated_positions.push(left_positions);
}

impl CorridorGeometryLibrary {
    /// Creates a new CorridorGeometryLibrary.
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Port of `CorridorGeometryLibrary.addAttribute`.
    pub fn add_attribute(
        attribute: &mut [f64],
        value: &Cartesian3,
        front: Option<usize>,
        back: Option<usize>,
    ) {
        let x = value.x;
        let y = value.y;
        let z = value.z;
        if let Some(front) = front {
            attribute[front] = x;
            attribute[front + 1] = y;
            attribute[front + 2] = z;
        }
        if let Some(back) = back {
            attribute[back] = z;
            attribute[back - 1] = y;
            attribute[back - 2] = x;
        }
    }

    /// Port of `CorridorGeometryLibrary.computePositions`.
    pub fn compute_positions(
        params: &CorridorComputePositionsParams,
    ) -> CorridorComputePositionsResult {
        let granularity = params.granularity;
        let positions = &params.positions;
        let ellipsoid = &params.ellipsoid;
        let width = params.width / 2.0;
        let corner_type = params.corner_type;
        let save_attributes = params.save_attributes;
        let mut calculated_positions: Vec<Vec<f64>> = Vec::new();
        let mut calculated_lefts: Option<Vec<f64>> = if save_attributes {
            Some(Vec::new())
        } else {
            None
        };
        let mut calculated_normals: Option<Vec<f64>> = if save_attributes {
            Some(Vec::new())
        } else {
            None
        };
        let position = positions[0]; // add first point
        let mut next_position = positions[1];

        let mut forward = Cartesian3::normalize_new(&Cartesian3::subtract_new(
            &next_position,
            &position,
        ));
        let mut normal = Cartesian3::default();
        ellipsoid.geodetic_surface_normal(&position, &mut normal);
        let mut left = Cartesian3::normalize_new(&Cartesian3::cross_new(&normal, &forward));
        if save_attributes {
            calculated_lefts
                .as_mut()
                .unwrap()
                .extend_from_slice(&[left.x, left.y, left.z]);
            calculated_normals
                .as_mut()
                .unwrap()
                .extend_from_slice(&[normal.x, normal.y, normal.z]);
        }
        let mut previous_pos = position;
        let mut position = next_position;
        let mut backward = Cartesian3::negate_new(&forward);

        let mut corners: Vec<CorridorCorner> = Vec::new();
        let length = positions.len();
        for i in 1..length - 1 {
            // add middle points and corners
            ellipsoid.geodetic_surface_normal(&position, &mut normal);
            next_position = positions[i + 1];
            forward = Cartesian3::normalize_new(&Cartesian3::subtract_new(
                &next_position,
                &position,
            ));

            let mut forward_projection = Cartesian3::multiply_by_scalar_new(
                &normal,
                Cartesian3::dot(&forward, &normal),
            );
            forward_projection = Cartesian3::subtract_new(&forward, &forward_projection);
            forward_projection = Cartesian3::normalize_new(&forward_projection);

            let mut backward_projection = Cartesian3::multiply_by_scalar_new(
                &normal,
                Cartesian3::dot(&backward, &normal),
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
                corner_direction = Cartesian3::cross_new(&corner_direction, &normal);
                corner_direction = Cartesian3::cross_new(&normal, &corner_direction);
                corner_direction = Cartesian3::normalize_new(&corner_direction);
                let scalar = width
                    / 0.25_f64.max(Cartesian3::magnitude(&Cartesian3::cross_new(
                        &corner_direction,
                        &backward,
                    )));
                let left_is_outside = PolylineVolumeGeometryLibrary::angle_is_greater_than_pi(
                    &forward, &backward, &position, ellipsoid,
                );
                let corner_direction =
                    Cartesian3::multiply_by_scalar_new(&corner_direction, scalar);
                if left_is_outside {
                    let right_pos = Cartesian3::add_new(&position, &corner_direction);
                    let center = Cartesian3::add_new(
                        &right_pos,
                        &Cartesian3::multiply_by_scalar_new(&left, width),
                    );
                    let mut left_pos = Cartesian3::add_new(
                        &right_pos,
                        &Cartesian3::multiply_by_scalar_new(&left, width * 2.0),
                    );
                    let subdivided_positions =
                        PolylinePipeline::generate_arc(Some(&GenerateArcOptions {
                            positions: vec![previous_pos, center],
                            granularity: Some(granularity),
                            ellipsoid: Some(ellipsoid.clone()),
                            ..Default::default()
                        }));
                    add_shifted_positions(
                        &subdivided_positions,
                        &left,
                        width,
                        &mut calculated_positions,
                    );
                    if save_attributes {
                        calculated_lefts
                            .as_mut()
                            .unwrap()
                            .extend_from_slice(&[left.x, left.y, left.z]);
                        calculated_normals
                            .as_mut()
                            .unwrap()
                            .extend_from_slice(&[normal.x, normal.y, normal.z]);
                    }
                    let start_point = left_pos;
                    left = Cartesian3::normalize_new(&Cartesian3::cross_new(&normal, &forward));
                    left_pos = Cartesian3::add_new(
                        &right_pos,
                        &Cartesian3::multiply_by_scalar_new(&left, width * 2.0),
                    );
                    previous_pos = Cartesian3::add_new(
                        &right_pos,
                        &Cartesian3::multiply_by_scalar_new(&left, width),
                    );
                    if corner_type == CornerType::Rounded || corner_type == CornerType::Beveled {
                        corners.push(CorridorCorner::LeftPositions(compute_round_corner(
                            &right_pos,
                            &start_point,
                            &left_pos,
                            corner_type,
                            left_is_outside,
                        )));
                    } else {
                        let negated_corner_direction = Cartesian3::negate_new(&corner_direction);
                        corners.push(CorridorCorner::LeftPositions(compute_mitered_corner(
                            &position,
                            &negated_corner_direction,
                            &left_pos,
                            left_is_outside,
                        )));
                    }
                } else {
                    let left_pos = Cartesian3::add_new(&position, &corner_direction);
                    let center = Cartesian3::add_new(
                        &left_pos,
                        &Cartesian3::negate_new(&Cartesian3::multiply_by_scalar_new(
                            &left, width,
                        )),
                    );
                    let mut right_pos = Cartesian3::add_new(
                        &left_pos,
                        &Cartesian3::negate_new(&Cartesian3::multiply_by_scalar_new(
                            &left,
                            width * 2.0,
                        )),
                    );
                    let subdivided_positions =
                        PolylinePipeline::generate_arc(Some(&GenerateArcOptions {
                            positions: vec![previous_pos, center],
                            granularity: Some(granularity),
                            ellipsoid: Some(ellipsoid.clone()),
                            ..Default::default()
                        }));
                    add_shifted_positions(
                        &subdivided_positions,
                        &left,
                        width,
                        &mut calculated_positions,
                    );
                    if save_attributes {
                        calculated_lefts
                            .as_mut()
                            .unwrap()
                            .extend_from_slice(&[left.x, left.y, left.z]);
                        calculated_normals
                            .as_mut()
                            .unwrap()
                            .extend_from_slice(&[normal.x, normal.y, normal.z]);
                    }
                    let start_point = right_pos;
                    left = Cartesian3::normalize_new(&Cartesian3::cross_new(&normal, &forward));
                    right_pos = Cartesian3::add_new(
                        &left_pos,
                        &Cartesian3::negate_new(&Cartesian3::multiply_by_scalar_new(
                            &left,
                            width * 2.0,
                        )),
                    );
                    previous_pos = Cartesian3::add_new(
                        &left_pos,
                        &Cartesian3::negate_new(&Cartesian3::multiply_by_scalar_new(
                            &left, width,
                        )),
                    );
                    if corner_type == CornerType::Rounded || corner_type == CornerType::Beveled {
                        corners.push(CorridorCorner::RightPositions(compute_round_corner(
                            &left_pos,
                            &start_point,
                            &right_pos,
                            corner_type,
                            left_is_outside,
                        )));
                    } else {
                        corners.push(CorridorCorner::RightPositions(compute_mitered_corner(
                            &position,
                            &corner_direction,
                            &right_pos,
                            left_is_outside,
                        )));
                    }
                }
                backward = Cartesian3::negate_new(&forward);
            }
            position = next_position;
        }

        ellipsoid.geodetic_surface_normal(&position, &mut normal);
        let subdivided_positions = PolylinePipeline::generate_arc(Some(&GenerateArcOptions {
            positions: vec![previous_pos, position],
            granularity: Some(granularity),
            ellipsoid: Some(ellipsoid.clone()),
            ..Default::default()
        }));
        add_shifted_positions(
            &subdivided_positions,
            &left,
            width,
            &mut calculated_positions,
        );
        if save_attributes {
            calculated_lefts
                .as_mut()
                .unwrap()
                .extend_from_slice(&[left.x, left.y, left.z]);
            calculated_normals
                .as_mut()
                .unwrap()
                .extend_from_slice(&[normal.x, normal.y, normal.z]);
        }

        let end_positions = if corner_type == CornerType::Rounded {
            Some(add_end_caps(&calculated_positions))
        } else {
            None
        };

        CorridorComputePositionsResult {
            positions: calculated_positions,
            corners,
            lefts: calculated_lefts,
            normals: calculated_normals,
            end_positions,
        }
    }
}

impl Default for CorridorGeometryLibrary {
    fn default() -> Self {
        Self::new()
    }
}
