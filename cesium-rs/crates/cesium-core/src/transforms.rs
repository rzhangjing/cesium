//! Ported from packages/engine/Source/Core/Transforms.js
//!
//! Contains functions for transforming positions to various reference frames.

use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;
use crate::heading_pitch_roll::HeadingPitchRoll;
use crate::math::CesiumMath;
use crate::matrix4::Matrix4;
use crate::quaternion::Quaternion;

/// Axis direction names for local reference frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AxisDirection {
    East,
    North,
    Up,
    West,
    South,
    Down,
}

/// Cross-product table: given first × second axis, returns the third axis.
fn vector_product_local_frame(first: AxisDirection, second: AxisDirection) -> Option<AxisDirection> {
    use AxisDirection::*;
    match (first, second) {
        // up × {south,north,west,east}
        (Up, South) => Some(East),
        (Up, North) => Some(West),
        (Up, West) => Some(South),
        (Up, East) => Some(North),
        // down × {south,north,west,east}
        (Down, South) => Some(West),
        (Down, North) => Some(East),
        (Down, West) => Some(South),
        (Down, East) => Some(North),
        // south × {up,down,west,east}
        (South, Up) => Some(West),
        (South, Down) => Some(East),
        (South, West) => Some(Down),
        (South, East) => Some(Up),
        // north × {up,down,west,east}
        (North, Up) => Some(East),
        (North, Down) => Some(West),
        (North, West) => Some(Up),
        (North, East) => Some(Down),
        // west × {up,down,north,south}
        (West, Up) => Some(North),
        (West, Down) => Some(South),
        (West, North) => Some(Down),
        (West, South) => Some(Up),
        // east × {up,down,north,south}
        (East, Up) => Some(South),
        (East, Down) => Some(North),
        (East, North) => Some(Up),
        (East, South) => Some(Down),
        // Degenerate / invalid combinations
        _ => None,
    }
}

/// Degenerate local frame vectors at the origin (position = 0,0,0).
fn degenerate_direction(dir: AxisDirection) -> Cartesian3 {
    use AxisDirection::*;
    match dir {
        North => Cartesian3::new(-1.0, 0.0, 0.0),
        East => Cartesian3::new(0.0, 1.0, 0.0),
        Up => Cartesian3::new(0.0, 0.0, 1.0),
        South => Cartesian3::new(1.0, 0.0, 0.0),
        West => Cartesian3::new(0.0, -1.0, 0.0),
        Down => Cartesian3::new(0.0, 0.0, -1.0),
    }
}

/// Returns true if the axis is East or West (not affected by pole sign).
fn is_east_west(dir: AxisDirection) -> bool {
    matches!(dir, AxisDirection::East | AxisDirection::West)
}

/// Port of `Transforms.localFrameToFixedFrameGenerator`.
///
/// Computes a 4×4 transformation matrix from a local reference frame
/// (defined by first and second axes) to the ellipsoid's fixed frame.
pub fn local_frame_to_fixed_frame(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
    first_axis: AxisDirection,
    second_axis: AxisDirection,
    result: &mut Matrix4,
) -> bool {
    let third_axis = match vector_product_local_frame(first_axis, second_axis) {
        Some(a) => a,
        None => return false,
    };

    let mut first;
    let mut second;
    let mut third;

    if Cartesian3::equals_epsilon(Some(origin), Some(&Cartesian3::ZERO), Some(CesiumMath::EPSILON14), None) {
        // Origin at center — use degenerate local frame
        first = degenerate_direction(first_axis);
        second = degenerate_direction(second_axis);
        third = degenerate_direction(third_axis);
    } else if CesiumMath::equals_epsilon(origin.x, 0.0, Some(CesiumMath::EPSILON14), None)
        && CesiumMath::equals_epsilon(origin.y, 0.0, Some(CesiumMath::EPSILON14), None)
    {
        // At a pole — special case
        let sign = CesiumMath::sign(origin.z);

        first = degenerate_direction(first_axis);
        if !is_east_west(first_axis) {
            first = Cartesian3::multiply_by_scalar_new(&first, sign);
        }

        second = degenerate_direction(second_axis);
        if !is_east_west(second_axis) {
            second = Cartesian3::multiply_by_scalar_new(&second, sign);
        }

        third = degenerate_direction(third_axis);
        if !is_east_west(third_axis) {
            third = Cartesian3::multiply_by_scalar_new(&third, sign);
        }
    } else {
        // Normal case — compute from geodetic surface normal
        let ell = ellipsoid.unwrap_or(&Ellipsoid::WGS84);
        let mut up = Cartesian3::default();
        ell.geodetic_surface_normal(origin, &mut up);

        let mut east = Cartesian3::new(-origin.y, origin.x, 0.0);
        east = Cartesian3::normalize_new(&east);
        let north = Cartesian3::cross_new(&up, &east);

        let down = Cartesian3::multiply_by_scalar_new(&up, -1.0);
        let west = Cartesian3::multiply_by_scalar_new(&east, -1.0);
        let south = Cartesian3::multiply_by_scalar_new(&north, -1.0);

        first = direction_vector(first_axis, &east, &north, &up, &west, &south, &down);
        second = direction_vector(second_axis, &east, &north, &up, &west, &south, &down);
        third = direction_vector(third_axis, &east, &north, &up, &west, &south, &down);
    }

    // Column-major storage: col0 = first, col1 = second, col2 = third, col3 = translation
    result.elements[0] = first.x;
    result.elements[1] = first.y;
    result.elements[2] = first.z;
    result.elements[3] = 0.0;
    result.elements[4] = second.x;
    result.elements[5] = second.y;
    result.elements[6] = second.z;
    result.elements[7] = 0.0;
    result.elements[8] = third.x;
    result.elements[9] = third.y;
    result.elements[10] = third.z;
    result.elements[11] = 0.0;
    result.elements[12] = origin.x;
    result.elements[13] = origin.y;
    result.elements[14] = origin.z;
    result.elements[15] = 1.0;
    true
}

fn direction_vector(
    dir: AxisDirection,
    east: &Cartesian3,
    north: &Cartesian3,
    up: &Cartesian3,
    west: &Cartesian3,
    south: &Cartesian3,
    down: &Cartesian3,
) -> Cartesian3 {
    match dir {
        AxisDirection::East => *east,
        AxisDirection::North => *north,
        AxisDirection::Up => *up,
        AxisDirection::West => *west,
        AxisDirection::South => *south,
        AxisDirection::Down => *down,
    }
}

/// Port of `Transforms.eastNorthUpToFixedFrame`.
pub fn east_north_up_to_fixed_frame(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
    result: &mut Matrix4,
) -> bool {
    local_frame_to_fixed_frame(origin, ellipsoid, AxisDirection::East, AxisDirection::North, result)
}

pub fn east_north_up_to_fixed_frame_new(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
) -> Matrix4 {
    let mut result = Matrix4::default();
    east_north_up_to_fixed_frame(origin, ellipsoid, &mut result);
    result
}

/// Port of `Transforms.northEastDownToFixedFrame`.
pub fn north_east_down_to_fixed_frame(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
    result: &mut Matrix4,
) -> bool {
    local_frame_to_fixed_frame(origin, ellipsoid, AxisDirection::North, AxisDirection::East, result)
}

pub fn north_east_down_to_fixed_frame_new(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
) -> Matrix4 {
    let mut result = Matrix4::default();
    north_east_down_to_fixed_frame(origin, ellipsoid, &mut result);
    result
}

/// Port of `Transforms.northUpEastToFixedFrame`.
pub fn north_up_east_to_fixed_frame(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
    result: &mut Matrix4,
) -> bool {
    local_frame_to_fixed_frame(origin, ellipsoid, AxisDirection::North, AxisDirection::Up, result)
}

pub fn north_up_east_to_fixed_frame_new(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
) -> Matrix4 {
    let mut result = Matrix4::default();
    north_up_east_to_fixed_frame(origin, ellipsoid, &mut result);
    result
}

/// Port of `Transforms.northWestUpToFixedFrame`.
pub fn north_west_up_to_fixed_frame(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
    result: &mut Matrix4,
) -> bool {
    local_frame_to_fixed_frame(origin, ellipsoid, AxisDirection::North, AxisDirection::West, result)
}

pub fn north_west_up_to_fixed_frame_new(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
) -> Matrix4 {
    let mut result = Matrix4::default();
    north_west_up_to_fixed_frame(origin, ellipsoid, &mut result);
    result
}

/// Port of `Transforms.headingPitchRollToFixedFrame`.
pub fn heading_pitch_roll_to_fixed_frame(
    origin: &Cartesian3,
    hpr: &HeadingPitchRoll,
    ellipsoid: Option<&Ellipsoid>,
    result: &mut Matrix4,
) -> bool {
    let hpr_quaternion = Quaternion::from_heading_pitch_roll_new(hpr);
    let scale = Cartesian3::new(1.0, 1.0, 1.0);
    let hpr_matrix = Matrix4::from_translation_quaternion_rotation_scale_new(
        &Cartesian3::ZERO,
        &hpr_quaternion,
        &scale,
    );
    if !east_north_up_to_fixed_frame(origin, ellipsoid, result) {
        return false;
    }
    let tmp = Matrix4::multiply_new(result, &hpr_matrix);
    *result = tmp;
    true
}

pub fn heading_pitch_roll_to_fixed_frame_new(
    origin: &Cartesian3,
    hpr: &HeadingPitchRoll,
    ellipsoid: Option<&Ellipsoid>,
) -> Matrix4 {
    let mut result = Matrix4::default();
    heading_pitch_roll_to_fixed_frame(origin, hpr, ellipsoid, &mut result);
    result
}

/// Port of `Transforms.headingPitchRollQuaternion`.
pub fn heading_pitch_roll_quaternion(
    origin: &Cartesian3,
    hpr: &HeadingPitchRoll,
    ellipsoid: Option<&Ellipsoid>,
    result: &mut Quaternion,
) -> bool {
    let mut transform = Matrix4::default();
    if !heading_pitch_roll_to_fixed_frame(origin, hpr, ellipsoid, &mut transform) {
        return false;
    }
    let rotation = Matrix4::get_matrix3_new(&transform);
    Quaternion::from_rotation_matrix(&rotation, result);
    true
}

pub fn heading_pitch_roll_quaternion_new(
    origin: &Cartesian3,
    hpr: &HeadingPitchRoll,
    ellipsoid: Option<&Ellipsoid>,
) -> Quaternion {
    let mut result = Quaternion::default();
    heading_pitch_roll_quaternion(origin, hpr, ellipsoid, &mut result);
    result
}

/// Port of `Transforms.fixedFrameToHeadingPitchRoll`.
pub fn fixed_frame_to_heading_pitch_roll(
    transform: &Matrix4,
    ellipsoid: Option<&Ellipsoid>,
    result: &mut HeadingPitchRoll,
) -> bool {
    let ell = ellipsoid.unwrap_or(&Ellipsoid::WGS84);
    let center = Matrix4::get_translation_new(transform);

    if Cartesian3::equals(Some(&center), Some(&Cartesian3::ZERO)) {
        result.heading = 0.0;
        result.pitch = 0.0;
        result.roll = 0.0;
        return true;
    }

    let mut ff = Matrix4::default();
    east_north_up_to_fixed_frame(&center, Some(ell), &mut ff);
    let mut to_fixed = Matrix4::default();
    Matrix4::inverse_transformation(&ff, &mut to_fixed);

    let no_scale = Cartesian3::new(1.0, 1.0, 1.0);
    let mut transform_copy = Matrix4::default();
    Matrix4::set_scale(transform, &no_scale, &mut transform_copy);
    let mut transform_copy2 = Matrix4::default();
    Matrix4::set_translation(&transform_copy, &Cartesian3::ZERO, &mut transform_copy2);

    let mut to_fixed_result = Matrix4::default();
    Matrix4::multiply(&to_fixed, &transform_copy2, &mut to_fixed_result);
    to_fixed = to_fixed_result;

    let rotation = Matrix4::get_matrix3_new(&to_fixed);
    let mut quat = Quaternion::default();
    Quaternion::from_rotation_matrix(&rotation, &mut quat);
    quat = Quaternion::normalize_new(&quat);

    HeadingPitchRoll::from_quaternion(&quat, result);
    true
}

pub fn fixed_frame_to_heading_pitch_roll_new(
    transform: &Matrix4,
    ellipsoid: Option<&Ellipsoid>,
) -> HeadingPitchRoll {
    let mut result = HeadingPitchRoll::default();
    fixed_frame_to_heading_pitch_roll(transform, ellipsoid, &mut result);
    result
}
