//! Coordinate transforms - ENU frames, HeadingPitchRoll, ICRF.
//! Maps to CesiumJS `Core/Transforms.js`, `Core/HeadingPitchRoll.js`, `Core/HeadingPitchRange.js`, `Core/TranslationRotationScale.js`

use crate::ellipsoid::Ellipsoid;
use crate::math_utils;
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
    /// Maps to `HeadingPitchRoll.toQuaternion` → `Quaternion.fromHeadingPitchRoll`
    pub fn to_quaternion(&self) -> DQuat {
        // ZYX intrinsic rotation: heading(Z) * pitch(Y) * roll(X)
        let cy = (self.heading * 0.5).cos();
        let sy = (self.heading * 0.5).sin();
        let cp = (self.pitch * 0.5).cos();
        let sp = (self.pitch * 0.5).sin();
        let cr = (self.roll * 0.5).cos();
        let sr = (self.roll * 0.5).sin();

        let w = cy * cp * cr + sy * sp * sr;
        let x = cy * cp * sr - sy * sp * cr;
        let y = cy * sp * cr + sy * cp * sr;
        let z = sy * cp * cr - cy * sp * sr;

        DQuat::from_xyzw(x, y, z, w)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// Computes the East-North-Up (ENU) reference frame at a given origin.
/// Maps to `Transforms.eastNorthUpToFixedFrame`
pub fn east_north_up_to_fixed_frame(origin: DVec3, ellipsoid: &Ellipsoid) -> DMat4 {
    local_frame_to_fixed_frame(Axis::X, Axis::Y, origin, ellipsoid)
}

/// Computes a local reference frame at a given origin.
/// Maps to `Transforms.localFrameToFixedFrameGenerator`
///
/// `first_axis` and `second_axis` define which geodetic directions map to X and Y.
/// For ENU: first=East(X), second=North(Y), third=Up(Z).
pub fn local_frame_to_fixed_frame(
    first_axis: Axis,
    second_axis: Axis,
    origin: DVec3,
    ellipsoid: &Ellipsoid,
) -> DMat4 {
    // Compute the geodetic surface normal (Up direction)
    let up = match ellipsoid.geodetic_surface_normal(origin) {
        Some(n) => n,
        None => DVec3::Z, // fallback at center
    };

    // East = normalize(cross(Z_world, up))
    let mut east = DVec3::Z.cross(up);
    if east.length_squared() < math_utils::EPSILON14 {
        // At poles, use X as reference
        east = DVec3::X.cross(up);
    }
    let east = east.normalize();

    // North = cross(up, east)
    let north = up.cross(east);

    // Build rotation columns based on axis assignment
    let (col0, col1, col2) = match (first_axis, second_axis) {
        (Axis::X, Axis::Y) => (east, north, up),       // ENU
        (Axis::X, Axis::Z) => (east, up, -north),      // EU(N flipped)
        (Axis::Y, Axis::X) => (north, east, -up),
        (Axis::Y, Axis::Z) => (north, up, east),
        (Axis::Z, Axis::X) => (up, east, north),
        (Axis::Z, Axis::Y) => (up, north, -east),
        _ => (east, north, up),
    };

    DMat4::from_cols(
        col0.extend(0.0),
        col1.extend(0.0),
        col2.extend(0.0),
        origin.extend(1.0),
    )
}

/// Computes a quaternion from heading/pitch/roll at a given origin on the ellipsoid.
/// Maps to `Transforms.headingPitchRollQuaternion`
pub fn heading_pitch_roll_quaternion(
    hpr: &HeadingPitchRoll,
    origin: DVec3,
    ellipsoid: &Ellipsoid,
) -> DQuat {
    let enu_matrix = east_north_up_to_fixed_frame(origin, ellipsoid);
    let enu_rotation = DMat3::from_cols(
        enu_matrix.x_axis.truncate(),
        enu_matrix.y_axis.truncate(),
        enu_matrix.z_axis.truncate(),
    );
    let enu_quat = DQuat::from_mat3(&enu_rotation);
    let hpr_quat = hpr.to_quaternion();
    enu_quat * hpr_quat
}

/// Computes a 4x4 matrix from heading/pitch/roll at a given origin.
/// Maps to `Transforms.headingPitchRollToFixedFrame`
pub fn heading_pitch_roll_to_fixed_frame(
    hpr: &HeadingPitchRoll,
    origin: DVec3,
    ellipsoid: &Ellipsoid,
) -> DMat4 {
    let quat = heading_pitch_roll_quaternion(hpr, origin, ellipsoid);
    let rotation = DMat3::from_quat(quat);
    DMat4::from_cols(
        rotation.x_axis.extend(0.0),
        rotation.y_axis.extend(0.0),
        rotation.z_axis.extend(0.0),
        origin.extend(1.0),
    )
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
        // 90° heading → rotation about Z by 90°
        assert!((quat.z - (PI / 4.0).sin()).abs() < 1e-10);
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
