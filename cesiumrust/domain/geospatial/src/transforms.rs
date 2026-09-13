//! Coordinate transforms - ENU frames, HeadingPitchRoll, ICRF.
//! Maps to CesiumJS `Core/Transforms.js`, `Core/HeadingPitchRoll.js`, `Core/HeadingPitchRange.js`, `Core/TranslationRotationScale.js`

use crate::ellipsoid::Ellipsoid;
use crate::math_utils;
use crate::projection::MapProjection;
use glam::{DMat3, DMat4, DQuat, DVec3};
use serde::{Deserialize, Serialize};

/// Heading, pitch, and roll angles (in radians).
/// Maps to CesiumJS `HeadingPitchRoll`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HeadingPitchRoll {
    /// Heading angle (rotation about the local Z/up axis), in radians.
    pub heading: f64,
    /// Pitch angle (rotation about the local Y/right axis), in radians.
    pub pitch: f64,
    /// Roll angle (rotation about the local X/forward axis), in radians.
    pub roll: f64,
}

impl Default for HeadingPitchRoll {
    fn default() -> Self {
        Self { heading: 0.0, pitch: 0.0, roll: 0.0 }
    }
}

impl HeadingPitchRoll {
    pub fn new(heading: f64, pitch: f64, roll: f64) -> Self {
        Self { heading, pitch, roll }
    }

    /// Creates from degrees.
    pub fn from_degrees(heading: f64, pitch: f64, roll: f64) -> Self {
        Self {
            heading: math_utils::to_radians(heading),
            pitch: math_utils::to_radians(pitch),
            roll: math_utils::to_radians(roll),
        }
    }

    /// Converts to a quaternion.
    /// Maps to `Quaternion.fromHeadingPitchRoll`:
    ///   heading(Z, -heading) * (pitch(Y, -pitch) * roll(X, +roll))
    ///
    /// Note the CesiumJS sign convention: heading and pitch are negated when
    /// converted to axis-angle rotations (heading is a rotation about the
    /// negative Z axis, pitch about the negative Y axis, roll about the
    /// positive X axis).
    pub fn to_quaternion(&self) -> DQuat {
        let roll = DQuat::from_axis_angle(DVec3::X, self.roll);
        let pitch = DQuat::from_axis_angle(DVec3::Y, -self.pitch);
        let heading = DQuat::from_axis_angle(DVec3::Z, -self.heading);
        heading * (pitch * roll)
    }

    /// Computes heading/pitch/roll from a quaternion.
    /// Maps to `HeadingPitchRoll.fromQuaternion`
    pub fn from_quaternion(quaternion: DQuat) -> Self {
        let test = 2.0 * (quaternion.w * quaternion.y - quaternion.z * quaternion.x);
        let denominator_roll =
            1.0 - 2.0 * (quaternion.x * quaternion.x + quaternion.y * quaternion.y);
        let numerator_roll = 2.0 * (quaternion.w * quaternion.x + quaternion.y * quaternion.z);
        let denominator_heading =
            1.0 - 2.0 * (quaternion.y * quaternion.y + quaternion.z * quaternion.z);
        let numerator_heading = 2.0 * (quaternion.w * quaternion.z + quaternion.x * quaternion.y);
        Self {
            heading: -numerator_heading.atan2(denominator_heading),
            pitch: -math_utils::clamp(test, -1.0, 1.0).asin(),
            roll: numerator_roll.atan2(denominator_roll),
        }
    }

    /// Compares with relative/absolute epsilon tolerance.
    /// Maps to `HeadingPitchRoll.equalsEpsilon`
    pub fn equals_epsilon(&self, other: &Self, relative_epsilon: f64) -> bool {
        fn eq_eps(left: f64, right: f64, rel_eps: f64) -> bool {
            let abs_diff = (left - right).abs();
            abs_diff <= rel_eps || abs_diff <= rel_eps * left.abs().max(right.abs())
        }
        eq_eps(self.heading, other.heading, relative_epsilon)
            && eq_eps(self.pitch, other.pitch, relative_epsilon)
            && eq_eps(self.roll, other.roll, relative_epsilon)
    }
}

impl std::fmt::Display for HeadingPitchRoll {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.heading, self.pitch, self.roll)
    }
}

/// Heading, pitch, and range (distance).
/// Maps to CesiumJS `HeadingPitchRange`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HeadingPitchRange {
    /// Heading angle in radians.
    pub heading: f64,
    /// Pitch angle in radians.
    pub pitch: f64,
    /// Range (distance) in meters.
    pub range: f64,
}

impl Default for HeadingPitchRange {
    fn default() -> Self {
        Self { heading: 0.0, pitch: 0.0, range: 0.0 }
    }
}

impl HeadingPitchRange {
    pub fn new(heading: f64, pitch: f64, range: f64) -> Self {
        Self { heading, pitch, range }
    }
}

/// Translation, rotation, and scale.
/// Maps to CesiumJS `TranslationRotationScale`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TranslationRotationScale {
    /// Translation vector.
    pub translation: DVec3,
    /// Rotation quaternion.
    pub rotation: DQuat,
    /// Scale vector.
    pub scale: DVec3,
}

impl Default for TranslationRotationScale {
    fn default() -> Self {
        Self {
            translation: DVec3::ZERO,
            rotation: DQuat::IDENTITY,
            scale: DVec3::ONE,
        }
    }
}

impl TranslationRotationScale {
    pub fn new(translation: DVec3, rotation: DQuat, scale: DVec3) -> Self {
        Self { translation, rotation, scale }
    }

    /// Converts to a 4x4 matrix.
    /// Maps to `Matrix4.fromTranslationRotationScale`
    pub fn to_matrix4(&self) -> DMat4 {
        let rotation_matrix = DMat3::from_quat(self.rotation);
        let scaled = DMat3::from_cols(
            rotation_matrix.x_axis * self.scale.x,
            rotation_matrix.y_axis * self.scale.y,
            rotation_matrix.z_axis * self.scale.z,
        );
        DMat4::from_cols(
            scaled.x_axis.extend(0.0),
            scaled.y_axis.extend(0.0),
            scaled.z_axis.extend(0.0),
            self.translation.extend(1.0),
        )
    }
}

/// Axis identifiers for local frame construction.
/// Maps to the string axis names ("east", "north", "up", "west", "south",
/// "down") accepted by CesiumJS `localFrameToFixedFrameGenerator`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocalFrameAxis {
    East,
    North,
    Up,
    West,
    South,
    Down,
}

use LocalFrameAxis::*;

/// The third axis completing a right-handed local frame (`first × second`).
/// Maps to CesiumJS `vectorProductLocalFrame`. Returns `None` when the two
/// axes are identical or opposite (CesiumJS rejects this with a DeveloperError).
fn third_axis(first: LocalFrameAxis, second: LocalFrameAxis) -> Option<LocalFrameAxis> {
    match (first, second) {
        (Up, South) => Some(East),
        (Up, North) => Some(West),
        (Up, West) => Some(South),
        (Up, East) => Some(North),
        (Down, South) => Some(West),
        (Down, North) => Some(East),
        (Down, West) => Some(North),
        (Down, East) => Some(South),
        (South, Up) => Some(West),
        (South, Down) => Some(East),
        (South, West) => Some(Down),
        (South, East) => Some(Up),
        (North, Up) => Some(East),
        (North, Down) => Some(West),
        (North, West) => Some(Up),
        (North, East) => Some(Down),
        (West, Up) => Some(North),
        (West, Down) => Some(South),
        (West, North) => Some(Down),
        (West, South) => Some(Up),
        (East, Up) => Some(South),
        (East, Down) => Some(North),
        (East, North) => Some(Up),
        (East, South) => Some(Down),
        _ => None,
    }
}

/// The local-frame axis vector in the degenerate case (ellipsoid center).
/// Maps to CesiumJS `degeneratePositionLocalFrame`.
fn degenerate_axis(axis: LocalFrameAxis) -> DVec3 {
    match axis {
        North => DVec3::new(-1.0, 0.0, 0.0),
        East => DVec3::new(0.0, 1.0, 0.0),
        Up => DVec3::new(0.0, 0.0, 1.0),
        South => DVec3::new(1.0, 0.0, 0.0),
        West => DVec3::new(0.0, -1.0, 0.0),
        Down => DVec3::new(0.0, 0.0, -1.0),
    }
}

/// Whether the axis is east or west (never sign-flipped at a pole).
fn is_east_west(axis: LocalFrameAxis) -> bool {
    matches!(axis, East | West)
}

/// Selects the concrete vector for a named axis from the six computed directions.
fn pick_axis(
    axis: LocalFrameAxis,
    east: DVec3,
    north: DVec3,
    up: DVec3,
    west: DVec3,
    south: DVec3,
    down: DVec3,
) -> DVec3 {
    match axis {
        East => east,
        North => north,
        Up => up,
        West => west,
        South => south,
        Down => down,
    }
}

/// Componentwise `Cartesian3.equalsEpsilon` (the original compares each
/// component with `CesiumMath.equalsEpsilon`, absolute epsilon defaulting to
/// the relative epsilon).
fn vec3_equals_epsilon(left: DVec3, right: DVec3, epsilon: f64) -> bool {
    math_utils::equals_epsilon(left.x, right.x, epsilon, epsilon)
        && math_utils::equals_epsilon(left.y, right.y, epsilon, epsilon)
        && math_utils::equals_epsilon(left.z, right.z, epsilon, epsilon)
}

/// Computes a local reference frame at a given origin.
/// Maps to `Transforms.localFrameToFixedFrameGenerator(firstAxis, secondAxis)`
///
/// `first_axis` and `second_axis` define which geodetic directions map to the
/// matrix's X and Y columns; the Z column is the right-handed third axis
/// (`first × second`).
///
/// Faithful port of the generated CesiumJS function:
/// - At the ellipsoid center: the degenerate local frame is used.
/// - At a pole (x and y both ~0): the degenerate frame is used, with every
///   non-east/west axis multiplied by `sign(z)`.
/// - Otherwise: up = geodetic surface normal, east = normalize(-origin.y,
///   origin.x, 0), north = up × east, and the opposites down/west/south.
pub fn local_frame_to_fixed_frame(
    first_axis: LocalFrameAxis,
    second_axis: LocalFrameAxis,
    origin: DVec3,
    ellipsoid: &Ellipsoid,
) -> DMat4 {
    let third = third_axis(first_axis, second_axis)
        .expect("firstAxis and secondAxis must be east, north, up, west, south or down.");

    let eps = math_utils::EPSILON14;
    let (first, second, third_vec) = if vec3_equals_epsilon(origin, DVec3::ZERO, eps) {
        // If x, y, and z are zero, use the degenerate local frame.
        (
            degenerate_axis(first_axis),
            degenerate_axis(second_axis),
            degenerate_axis(third),
        )
    } else if math_utils::equals_epsilon(origin.x, 0.0, eps, eps)
        && math_utils::equals_epsilon(origin.y, 0.0, eps, eps)
    {
        // If x and y are zero, assume origin is at a pole.
        let sign = math_utils::sign(origin.z);
        let mut first = degenerate_axis(first_axis);
        if !is_east_west(first_axis) {
            first *= sign;
        }
        let mut second = degenerate_axis(second_axis);
        if !is_east_west(second_axis) {
            second *= sign;
        }
        let mut third_vec = degenerate_axis(third);
        if !is_east_west(third) {
            third_vec *= sign;
        }
        (first, second, third_vec)
    } else {
        // General position.
        let up = ellipsoid
            .geodetic_surface_normal(origin)
            .expect("origin must not be at the center of the ellipsoid");
        let east = crate::ellipsoid::normalize_cartesian3(DVec3::new(-origin.y, origin.x, 0.0));
        let north = up.cross(east);
        let down = -up;
        let west = -east;
        let south = -north;
        (
            pick_axis(first_axis, east, north, up, west, south, down),
            pick_axis(second_axis, east, north, up, west, south, down),
            pick_axis(third, east, north, up, west, south, down),
        )
    };

    DMat4::from_cols(
        first.extend(0.0),
        second.extend(0.0),
        third_vec.extend(0.0),
        origin.extend(1.0),
    )
}

/// Computes the East-North-Up (ENU) reference frame at a given origin.
/// Maps to `Transforms.eastNorthUpToFixedFrame`
pub fn east_north_up_to_fixed_frame(origin: DVec3, ellipsoid: &Ellipsoid) -> DMat4 {
    local_frame_to_fixed_frame(East, North, origin, ellipsoid)
}

/// Computes the North-East-Down (NED) reference frame at a given origin.
/// Maps to `Transforms.northEastDownToFixedFrame`
pub fn north_east_down_to_fixed_frame(origin: DVec3, ellipsoid: &Ellipsoid) -> DMat4 {
    local_frame_to_fixed_frame(North, East, origin, ellipsoid)
}

/// Computes the North-Up-East (NUE) reference frame at a given origin.
/// Maps to `Transforms.northUpEastToFixedFrame`
pub fn north_up_east_to_fixed_frame(origin: DVec3, ellipsoid: &Ellipsoid) -> DMat4 {
    local_frame_to_fixed_frame(North, Up, origin, ellipsoid)
}

/// Computes the North-West-Up (NWU) reference frame at a given origin.
/// Maps to `Transforms.northWestUpToFixedFrame`
pub fn north_west_up_to_fixed_frame(origin: DVec3, ellipsoid: &Ellipsoid) -> DMat4 {
    local_frame_to_fixed_frame(North, West, origin, ellipsoid)
}

/// Computes a 4x4 matrix from heading/pitch/roll at a given origin, using the
/// default East-North-Up local frame.
/// Maps to `Transforms.headingPitchRollToFixedFrame`
pub fn heading_pitch_roll_to_fixed_frame(
    hpr: &HeadingPitchRoll,
    origin: DVec3,
    ellipsoid: &Ellipsoid,
) -> DMat4 {
    heading_pitch_roll_to_fixed_frame_with_local_frame(hpr, origin, ellipsoid, East, North)
}

/// Computes a 4x4 matrix from heading/pitch/roll at a given origin, using a
/// custom local frame defined by `first_axis`/`second_axis`.
/// Maps to `Transforms.headingPitchRollToFixedFrame` with a custom
/// `fixedFrameTransform`.
///
/// Faithful port: builds the local-frame-to-fixed matrix, then multiplies by
/// the heading/pitch/roll rotation matrix (as a rigid transform), matching
/// CesiumJS `Matrix4.multiply(fixedFrame, hprMatrix)`.
pub fn heading_pitch_roll_to_fixed_frame_with_local_frame(
    hpr: &HeadingPitchRoll,
    origin: DVec3,
    ellipsoid: &Ellipsoid,
    first_axis: LocalFrameAxis,
    second_axis: LocalFrameAxis,
) -> DMat4 {
    let fixed_frame = local_frame_to_fixed_frame(first_axis, second_axis, origin, ellipsoid);
    let hpr_rotation = DMat3::from_quat(hpr.to_quaternion());
    let hpr_matrix = DMat4::from_cols(
        hpr_rotation.x_axis.extend(0.0),
        hpr_rotation.y_axis.extend(0.0),
        hpr_rotation.z_axis.extend(0.0),
        DVec3::ZERO.extend(1.0),
    );
    fixed_frame * hpr_matrix
}

/// Computes a quaternion from heading/pitch/roll at a given origin, using the
/// default East-North-Up local frame.
/// Maps to `Transforms.headingPitchRollQuaternion`
pub fn heading_pitch_roll_quaternion(
    hpr: &HeadingPitchRoll,
    origin: DVec3,
    ellipsoid: &Ellipsoid,
) -> DQuat {
    heading_pitch_roll_quaternion_with_local_frame(hpr, origin, ellipsoid, East, North)
}

/// Computes a quaternion from heading/pitch/roll at a given origin, using a
/// custom local frame.
/// Maps to `Transforms.headingPitchRollQuaternion` with a custom
/// `fixedFrameTransform`.
pub fn heading_pitch_roll_quaternion_with_local_frame(
    hpr: &HeadingPitchRoll,
    origin: DVec3,
    ellipsoid: &Ellipsoid,
    first_axis: LocalFrameAxis,
    second_axis: LocalFrameAxis,
) -> DQuat {
    let transform = heading_pitch_roll_to_fixed_frame_with_local_frame(
        hpr,
        origin,
        ellipsoid,
        first_axis,
        second_axis,
    );
    let rotation = DMat3::from_cols(
        transform.x_axis.truncate(),
        transform.y_axis.truncate(),
        transform.z_axis.truncate(),
    );
    DQuat::from_mat3(&rotation)
}

/// Computes the rotation matrix from ICRF (inertial) to fixed frame.
/// Simplified: uses Earth rotation angle approximation.
/// Maps to `Transforms.computeIcrfToFixedMatrix`
pub fn compute_icrf_to_fixed_matrix(julian_date_seconds: f64) -> Option<DMat3> {
    // Simplified Earth rotation: GMST approximation
    // Full implementation would use IAU 2006/2000A precession-nutation
    let days_since_j2000 = julian_date_seconds / 86400.0 - 2451545.0;
    let gmst = math_utils::zero_to_two_pi(
        math_utils::to_radians(280.46061837 + 360.98564736629 * days_since_j2000),
    );

    let cos_gmst = gmst.cos();
    let sin_gmst = gmst.sin();

    // Rotation about Z axis by GMST
    Some(DMat3::from_cols_array(&[
        cos_gmst, -sin_gmst, 0.0,
        sin_gmst, cos_gmst, 0.0,
        0.0, 0.0, 1.0,
    ]))
}

/// Computes the rotation matrix from fixed frame to ICRF (inertial).
/// Maps to `Transforms.computeFixedToIcrfMatrix`
pub fn compute_fixed_to_icrf_matrix(julian_date_seconds: f64) -> Option<DMat3> {
    compute_icrf_to_fixed_matrix(julian_date_seconds).map(|m| m.transpose())
}

/// Computes a view matrix looking at a target from a position.
/// Maps to `Transforms.lookAt` (simplified)
pub fn look_at(eye: DVec3, target: DVec3, up: DVec3) -> DMat4 {
    let z_axis = (eye - target).normalize();
    let x_axis = up.cross(z_axis).normalize();
    let y_axis = z_axis.cross(x_axis);

    DMat4::from_cols(
        x_axis.extend(0.0),
        y_axis.extend(0.0),
        z_axis.extend(0.0),
        eye.extend(1.0),
    )
}

/// Computes a rotation matrix from a position and velocity (flight direction).
/// Maps to `Transforms.rotationMatrixFromPositionVelocity`
///
/// The resulting matrix's columns are `[velocity, right, up]`, matching the
/// CesiumJS implementation which assigns `result[0..2]=velocity,
/// result[3..5]=right, result[6..8]=up` (column-major storage).
pub fn rotation_matrix_from_position_velocity(
    position: DVec3,
    velocity: DVec3,
    ellipsoid: &Ellipsoid,
) -> DMat3 {
    let normal = ellipsoid
        .geodetic_surface_normal(position)
        .expect("position must not be at the center of the ellipsoid");

    let mut right = velocity.cross(normal);
    if vec3_equals_epsilon(right, DVec3::ZERO, math_utils::EPSILON6) {
        right = DVec3::X;
    }

    let up = crate::ellipsoid::normalize_cartesian3(right.cross(velocity));
    right = -velocity.cross(up);
    right = crate::ellipsoid::normalize_cartesian3(right);

    DMat3::from_cols(velocity, right, up)
}

/// Inverts a rigid (orthonormal rotation + translation) transformation.
/// Maps to `Matrix4.inverseTransformation`: `[R^T | -R^T * t]`.
pub fn inverse_transformation(matrix: &DMat4) -> DMat4 {
    let rotation = DMat3::from_cols(
        matrix.x_axis.truncate(),
        matrix.y_axis.truncate(),
        matrix.z_axis.truncate(),
    );
    let rotation_t = rotation.transpose();
    let new_translation = -(rotation_t * matrix.w_axis.truncate());
    DMat4::from_cols(
        rotation_t.x_axis.extend(0.0),
        rotation_t.y_axis.extend(0.0),
        rotation_t.z_axis.extend(0.0),
        new_translation.extend(1.0),
    )
}

/// The swizzle matrix mapping (x, y, z) -> (z, x, y), used to convert a 3D
/// ENU frame into a 2D projected frame.
/// Maps to `Transforms.SWIZZLE_3D_TO_2D_MATRIX` (column-major columns [Y, Z, X]).
fn swizzle_3d_to_2d_matrix() -> DMat4 {
    DMat4::from_cols(
        DVec3::Y.extend(0.0),
        DVec3::Z.extend(0.0),
        DVec3::X.extend(0.0),
        DVec3::ZERO.extend(1.0),
    )
}

/// Computes heading/pitch/roll angles from a transform in the fixed frame.
/// Maps to `Transforms.fixedFrameToHeadingPitchRoll`
pub fn fixed_frame_to_heading_pitch_roll(
    transform: &DMat4,
    ellipsoid: &Ellipsoid,
) -> HeadingPitchRoll {
    let center = transform.w_axis.truncate();
    if center == DVec3::ZERO {
        return HeadingPitchRoll::new(0.0, 0.0, 0.0);
    }
    let to_fixed_frame = inverse_transformation(&east_north_up_to_fixed_frame(center, ellipsoid));

    // Matrix4.setScale(transform, (1,1,1)): normalize each rotation column
    // (divide xyz by its length), preserving the w component; then
    // Matrix4.setTranslation(.., ZERO).
    let mut transform_copy = *transform;
    let x_scale = transform.x_axis.truncate().length();
    let y_scale = transform.y_axis.truncate().length();
    let z_scale = transform.z_axis.truncate().length();
    transform_copy.x_axis = (transform.x_axis.truncate() / x_scale).extend(transform.x_axis.w);
    transform_copy.y_axis = (transform.y_axis.truncate() / y_scale).extend(transform.y_axis.w);
    transform_copy.z_axis = (transform.z_axis.truncate() / z_scale).extend(transform.z_axis.w);
    transform_copy.w_axis = DVec3::ZERO.extend(1.0);

    let to_fixed_frame = to_fixed_frame * transform_copy;
    let rotation = DMat3::from_cols(
        to_fixed_frame.x_axis.truncate(),
        to_fixed_frame.y_axis.truncate(),
        to_fixed_frame.z_axis.truncate(),
    );
    HeadingPitchRoll::from_quaternion(DQuat::from_mat3(&rotation).normalize())
}

/// Computes a 2D transformation from a 3D basis using the given projection.
/// Maps to `Transforms.basisTo2D`
pub fn basis_to_2d<P: MapProjection>(projection: &P, matrix: &DMat4) -> DMat4 {
    let rtc_center = matrix.w_axis.truncate();
    let ellipsoid = *projection.ellipsoid();

    let projected_position = if rtc_center == DVec3::ZERO {
        DVec3::ZERO
    } else {
        let cartographic = ellipsoid
            .cartesian_to_cartographic(rtc_center)
            .expect("rtcCenter must be on or above the ellipsoid");
        let p = projection.project(&cartographic);
        DVec3::new(p.z, p.x, p.y)
    };

    let from_enu = east_north_up_to_fixed_frame(rtc_center, &ellipsoid);
    let to_enu = inverse_transformation(&from_enu);
    let rotation = DMat3::from_cols(
        matrix.x_axis.truncate(),
        matrix.y_axis.truncate(),
        matrix.z_axis.truncate(),
    );
    let local = to_enu
        * DMat4::from_cols(
            rotation.x_axis.extend(0.0),
            rotation.y_axis.extend(0.0),
            rotation.z_axis.extend(0.0),
            DVec3::ZERO.extend(1.0),
        );
    let mut result = swizzle_3d_to_2d_matrix() * local;
    result.w_axis = projected_position.extend(1.0);
    result
}

/// Computes a 2D model matrix from a 3D ellipsoid-centered frame.
/// Maps to `Transforms.ellipsoidTo2DModelMatrix`
pub fn ellipsoid_to_2d_model_matrix<P: MapProjection>(projection: &P, center: DVec3) -> DMat4 {
    let ellipsoid = *projection.ellipsoid();
    let from_enu = east_north_up_to_fixed_frame(center, &ellipsoid);
    let to_enu = inverse_transformation(&from_enu);
    let cartographic = ellipsoid
        .cartesian_to_cartographic(center)
        .expect("center must be on or above the ellipsoid");
    let p = projection.project(&cartographic);
    let projected_position = DVec3::new(p.z, p.x, p.y);
    let translation = DMat4::from_translation(projected_position);
    translation * (swizzle_3d_to_2d_matrix() * to_enu)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_enu_at_equator_prime_meridian() {
        let ellipsoid = Ellipsoid::WGS84;
        let origin = DVec3::new(6378137.0, 0.0, 0.0);
        let frame = east_north_up_to_fixed_frame(origin, &ellipsoid);

        // At (lat=0, lon=0): East = (0,1,0), North = (0,0,1), Up = (1,0,0)
        let east = frame.x_axis.truncate();
        let north = frame.y_axis.truncate();
        let up = frame.z_axis.truncate();

        assert!(east.abs_diff_eq(DVec3::Y, 1e-10), "East: {:?}", east);
        assert!(north.abs_diff_eq(DVec3::Z, 1e-10), "North: {:?}", north);
        assert!(up.abs_diff_eq(DVec3::X, 1e-10), "Up: {:?}", up);
    }

    #[test]
    #[allow(clippy::excessive_precision)]
    fn test_enu_at_north_pole() {
        let ellipsoid = Ellipsoid::WGS84;
        let origin = DVec3::new(0.0, 0.0, 6356752.3142451793);
        let frame = east_north_up_to_fixed_frame(origin, &ellipsoid);

        // At north pole: Up = (0,0,1)
        let up = frame.z_axis.truncate();
        assert!(up.abs_diff_eq(DVec3::Z, 1e-10), "Up at pole: {:?}", up);
    }

    #[test]
    fn test_heading_pitch_roll_quaternion_identity() {
        let hpr = HeadingPitchRoll::new(0.0, 0.0, 0.0);
        let quat = hpr.to_quaternion();
        assert!((quat.w - 1.0).abs() < 1e-10);
        assert!(quat.x.abs() < 1e-10);
        assert!(quat.y.abs() < 1e-10);
        assert!(quat.z.abs() < 1e-10);
    }

    #[test]
    fn test_heading_pitch_roll_heading_90() {
        let hpr = HeadingPitchRoll::new(PI / 2.0, 0.0, 0.0);
        let quat = hpr.to_quaternion();
        // 90° heading → rotation about -Z by 90° (CesiumJS convention), so
        // z = -sin(PI/4), w = cos(PI/4).
        assert!((quat.z + (PI / 4.0).sin()).abs() < 1e-10);
        assert!((quat.w - (PI / 4.0).cos()).abs() < 1e-10);
    }

    #[test]
    fn test_translation_rotation_scale_to_matrix() {
        let trs = TranslationRotationScale::new(
            DVec3::new(1.0, 2.0, 3.0),
            DQuat::IDENTITY,
            DVec3::ONE,
        );
        let mat = trs.to_matrix4();
        assert_eq!(mat.w_axis.truncate(), DVec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_icrf_to_fixed() {
        // At J2000 epoch, GMST ≈ 280.46° → rotation should be non-identity
        let j2000_seconds = 2451545.0 * 86400.0;
        let mat = compute_icrf_to_fixed_matrix(j2000_seconds).unwrap();
        // Should be a valid rotation matrix (det ≈ 1)
        let det = mat.determinant();
        assert!((det - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_fixed_to_icrf_is_inverse() {
        let seconds = 2451545.0 * 86400.0 + 3600.0;
        let icrf_to_fixed = compute_icrf_to_fixed_matrix(seconds).unwrap();
        let fixed_to_icrf = compute_fixed_to_icrf_matrix(seconds).unwrap();
        let product = icrf_to_fixed * fixed_to_icrf;
        // Should be identity
        assert!(product.abs_diff_eq(DMat3::IDENTITY, 1e-10));
    }
}
