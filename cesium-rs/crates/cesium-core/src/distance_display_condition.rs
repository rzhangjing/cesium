//! Ported from `packages/engine/Source/Core/DistanceDisplayCondition.js`.

/// Determines visibility based on the distance to the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistanceDisplayCondition {
    /// The smallest distance where the object is visible.
    pub near: f64,
    /// The largest distance where the object is visible.
    pub far: f64,
}

impl Default for DistanceDisplayCondition {
    fn default() -> Self {
        Self {
            near: 0.0,
            far: f64::MAX,
        }
    }
}

impl DistanceDisplayCondition {
    pub fn new(near: f64, far: f64) -> Self {
        Self { near, far }
    }

    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize = 2;

    /// Stores the provided instance into the provided array.
    pub fn pack(value: &Self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = value.near;
        array[starting_index + 1] = value.far;
    }

    /// Retrieves an instance from a packed array.
    pub fn unpack(array: &[f64], starting_index: usize) -> Self {
        Self {
            near: array[starting_index],
            far: array[starting_index + 1],
        }
    }

    /// Determines if two distance display conditions are equal.
    pub fn equals(left: &Self, right: &Self) -> bool {
        left.near == right.near && left.far == right.far
    }
}
