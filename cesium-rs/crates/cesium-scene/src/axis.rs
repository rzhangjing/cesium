//! Ported from `packages/engine/Source/Scene/Axis.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;

/// An enum describing the x, y, and z axes and helper conversion functions.
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
    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Creates from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::X),
            1 => Some(Self::Y),
            2 => Some(Self::Z),
            _ => None,
        }
    }

    fn origin() -> Cartesian3 {
        Cartesian3::from_elements_new(0.0, 0.0, 0.0)
    }

    /// Matrix used to convert from y-up to z-up (rotation about PI/2 around the X-axis).
    pub fn y_up_to_z_up() -> Matrix4 {
        let rotation = Matrix3::from_column_major_array_new(&[
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0,
        ]);
        Matrix4::from_rotation_translation_new(&rotation, &Self::origin())
    }

    /// Matrix used to convert from z-up to y-up (rotation about -PI/2 around the X-axis).
    pub fn z_up_to_y_up() -> Matrix4 {
        let rotation = Matrix3::from_column_major_array_new(&[
            1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0,
        ]);
        Matrix4::from_rotation_translation_new(&rotation, &Self::origin())
    }

    /// Matrix used to convert from x-up to y-up (rotation about PI/2 around the Z-axis).
    pub fn x_up_to_y_up() -> Matrix4 {
        let rotation = Matrix3::from_column_major_array_new(&[
            0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]);
        Matrix4::from_rotation_translation_new(&rotation, &Self::origin())
    }

    /// Matrix used to convert from x-up to z-up.
    pub fn x_up_to_z_up() -> Matrix4 {
        let rotation = Matrix3::from_column_major_array_new(&[
            0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0,
        ]);
        Matrix4::from_rotation_translation_new(&rotation, &Self::origin())
    }

    /// Matrix used to convert from z-up to x-up.
    pub fn z_up_to_x_up() -> Matrix4 {
        let rotation = Matrix3::from_column_major_array_new(&[
            0.0, -1.0, 0.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0,
        ]);
        Matrix4::from_rotation_translation_new(&rotation, &Self::origin())
    }

    /// Matrix used to convert from y-up to x-up.
    pub fn y_up_to_x_up() -> Matrix4 {
        let rotation = Matrix3::from_column_major_array_new(&[
            0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]);
        Matrix4::from_rotation_translation_new(&rotation, &Self::origin())
    }
}
