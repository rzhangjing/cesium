//! Axis conversion matrices for glTF up-axis handling.
//!
//! Maps to CesiumJS `Scene/Axis.js`

use glam::DMat4;

/// An enum describing the x, y, and z axes and helper conversion functions.
///
/// Maps to CesiumJS `Scene/Axis.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Axis {
    /// Denotes the x-axis.
    X = 0,
    /// Denotes the y-axis.
    Y = 1,
    /// Denotes the z-axis.
    Z = 2,
}

impl Axis {
    /// Gets the axis by name.
    ///
    /// Maps to CesiumJS `Axis.fromName`.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "X" => Some(Axis::X),
            "Y" => Some(Axis::Y),
            "Z" => Some(Axis::Z),
            _ => None,
        }
    }
}

/// Matrix used to convert from y-up to z-up.
/// Rotation about PI/2 around the X-axis.
///
/// Maps to CesiumJS `Axis.Y_UP_TO_Z_UP`.
pub const Y_UP_TO_Z_UP: DMat4 = DMat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, // column 0
    0.0, 0.0, 1.0, 0.0, // column 1
    0.0, -1.0, 0.0, 0.0, // column 2
    0.0, 0.0, 0.0, 1.0, // column 3
]);

/// Matrix used to convert from z-up to y-up.
/// Rotation about -PI/2 around the X-axis.
///
/// Maps to CesiumJS `Axis.Z_UP_TO_Y_UP`.
pub const Z_UP_TO_Y_UP: DMat4 = DMat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, // column 0
    0.0, 0.0, -1.0, 0.0, // column 1
    0.0, 1.0, 0.0, 0.0, // column 2
    0.0, 0.0, 0.0, 1.0, // column 3
]);

/// Matrix used to convert from x-up to z-up.
/// Rotation about -PI/2 around the Y-axis.
///
/// Maps to CesiumJS `Axis.X_UP_TO_Z_UP`.
pub const X_UP_TO_Z_UP: DMat4 = DMat4::from_cols_array(&[
    0.0, 0.0, 1.0, 0.0, // column 0
    0.0, 1.0, 0.0, 0.0, // column 1
    -1.0, 0.0, 0.0, 0.0, // column 2
    0.0, 0.0, 0.0, 1.0, // column 3
]);

/// Matrix used to convert from z-up to x-up.
/// Rotation about PI/2 around the Y-axis.
///
/// Maps to CesiumJS `Axis.Z_UP_TO_X_UP`.
pub const Z_UP_TO_X_UP: DMat4 = DMat4::from_cols_array(&[
    0.0, 0.0, -1.0, 0.0, // column 0
    0.0, 1.0, 0.0, 0.0, // column 1
    1.0, 0.0, 0.0, 0.0, // column 2
    0.0, 0.0, 0.0, 1.0, // column 3
]);

/// Matrix used to convert from x-up to y-up.
/// Rotation about PI/2 around the Z-axis.
///
/// Maps to CesiumJS `Axis.X_UP_TO_Y_UP`.
pub const X_UP_TO_Y_UP: DMat4 = DMat4::from_cols_array(&[
    0.0, 1.0, 0.0, 0.0, // column 0
    -1.0, 0.0, 0.0, 0.0, // column 1
    0.0, 0.0, 1.0, 0.0, // column 2
    0.0, 0.0, 0.0, 1.0, // column 3
]);

/// Matrix used to convert from y-up to x-up.
/// Rotation about -PI/2 around the Z-axis.
///
/// Maps to CesiumJS `Axis.Y_UP_TO_X_UP`.
pub const Y_UP_TO_X_UP: DMat4 = DMat4::from_cols_array(&[
    0.0, -1.0, 0.0, 0.0, // column 0
    1.0, 0.0, 0.0, 0.0, // column 1
    0.0, 0.0, 1.0, 0.0, // column 2
    0.0, 0.0, 0.0, 1.0, // column 3
]);
