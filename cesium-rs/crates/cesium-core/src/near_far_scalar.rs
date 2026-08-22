//! Ported from `packages/engine/Source/Core/NearFarScalar.js`.

/// Represents a scalar value's lower and upper bound at a near distance
/// and far distance in eye space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearFarScalar {
    /// The lower bound of the camera range.
    pub near: f64,
    /// The value at the lower bound of the camera range.
    pub near_value: f64,
    /// The upper bound of the camera range.
    pub far: f64,
    /// The value at the upper bound of the camera range.
    pub far_value: f64,
}

impl Default for NearFarScalar {
    fn default() -> Self {
        Self {
            near: 0.0,
            near_value: 0.0,
            far: 1.0,
            far_value: 0.0,
        }
    }
}

impl NearFarScalar {
    pub fn new(near: f64, near_value: f64, far: f64, far_value: f64) -> Self {
        Self {
            near,
            near_value,
            far,
            far_value,
        }
    }

    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize = 4;

    /// Stores the provided instance into the provided array.
    pub fn pack(value: &Self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = value.near;
        array[starting_index + 1] = value.near_value;
        array[starting_index + 2] = value.far;
        array[starting_index + 3] = value.far_value;
    }

    /// Retrieves an instance from a packed array.
    pub fn unpack(array: &[f64], starting_index: usize) -> Self {
        Self {
            near: array[starting_index],
            near_value: array[starting_index + 1],
            far: array[starting_index + 2],
            far_value: array[starting_index + 3],
        }
    }

    /// Compares two NearFarScalar instances for equality.
    pub fn equals(left: &Self, right: &Self) -> bool {
        left.near == right.near
            && left.near_value == right.near_value
            && left.far == right.far
            && left.far_value == right.far_value
    }
}
