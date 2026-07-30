//! Cartesian2 CesiumJS extension functions.
//! Maps to CesiumJS `Core/Cartesian2.js` static methods that go beyond basic vector math.

use crate::math_utils;
use glam::DVec2;

/// The packed length of a Cartesian2: 2.
pub const PACKED_LENGTH: usize = 2;

/// Packs a Cartesian2 into an array at the given starting index.
/// Maps to CesiumJS `Cartesian2.pack`
pub fn pack(value: DVec2, array: &mut [f64], starting_index: usize) {
    array[starting_index] = value.x;
    array[starting_index + 1] = value.y;
}

/// Unpacks a Cartesian2 from an array at the given starting index.
/// Maps to CesiumJS `Cartesian2.unpack`
pub fn unpack(array: &[f64], starting_index: usize) -> DVec2 {
    DVec2::new(array[starting_index], array[starting_index + 1])
}

/// Flattens an array of Cartesian2s into an array of components.
/// Maps to CesiumJS `Cartesian2.packArray`
pub fn pack_array(array: &[DVec2]) -> Vec<f64> {
    let length = array.len();
    let mut result = vec![0.0f64; length * 2];
    for i in 0..length {
        pack(array[i], &mut result, i * 2);
    }
    result
}

/// Unpacks an array of components into an array of Cartesian2s.
/// Maps to CesiumJS `Cartesian2.unpackArray`
pub fn unpack_array(array: &[f64]) -> Vec<DVec2> {
    let length = array.len() / 2;
    let mut result = Vec::with_capacity(length);
    for i in 0..length {
        result.push(unpack(array, i * 2));
    }
    result
}

/// Creates a Cartesian2 from the first two elements of an array at an offset.
/// Maps to CesiumJS `Cartesian2.fromArray`
pub fn from_array(array: &[f64], starting_index: usize) -> DVec2 {
    DVec2::new(array[starting_index], array[starting_index + 1])
}

/// Returns the component with the maximum value.
/// Maps to CesiumJS `Cartesian2.maximumComponent`
pub fn maximum_component(cartesian: DVec2) -> f64 {
    cartesian.x.max(cartesian.y)
}

/// Returns the component with the minimum value.
/// Maps to CesiumJS `Cartesian2.minimumComponent`
pub fn minimum_component(cartesian: DVec2) -> f64 {
    cartesian.x.min(cartesian.y)
}

/// Computes the provided Cartesian's squared magnitude.
/// Maps to CesiumJS `Cartesian2.magnitudeSquared`
pub fn magnitude_squared(cartesian: DVec2) -> f64 {
    cartesian.x * cartesian.x + cartesian.y * cartesian.y
}

/// Computes the Cartesian's magnitude (length).
/// Maps to CesiumJS `Cartesian2.magnitude`
pub fn magnitude(cartesian: DVec2) -> f64 {
    magnitude_squared(cartesian).sqrt()
}

/// Computes the 2D cross product of two vectors (returns scalar z-component).
/// Maps to CesiumJS `Cartesian2.cross`
pub fn cross(left: DVec2, right: DVec2) -> f64 {
    left.x * right.y - left.y * right.x
}

/// Computes the distance between two points.
/// Maps to CesiumJS `Cartesian2.distance`
pub fn distance(left: DVec2, right: DVec2) -> f64 {
    (left - right).length()
}

/// Computes the squared distance between two points.
/// Maps to CesiumJS `Cartesian2.distanceSquared`
pub fn distance_squared(left: DVec2, right: DVec2) -> f64 {
    (left - right).length_squared()
}

/// Computes the linear interpolation or extrapolation at t using the provided cartesians.
/// Maps to CesiumJS `Cartesian2.lerp`
pub fn lerp(start: DVec2, end: DVec2, t: f64) -> DVec2 {
    start + (end - start) * t
}

/// Computes the angle between two vectors.
/// Maps to CesiumJS `Cartesian2.angleBetween`
pub fn angle_between(left: DVec2, right: DVec2) -> f64 {
    let cross_val = cross(left, right);
    let dot_val = left.dot(right);
    cross_val.abs().atan2(dot_val)
}

/// Returns the axis that is most orthogonal to the provided Cartesian.
/// Maps to CesiumJS `Cartesian2.mostOrthogonalAxis`
pub fn most_orthogonal_axis(cartesian: DVec2) -> DVec2 {
    let f = cartesian.normalize_or_zero();
    let f = DVec2::new(f.x.abs(), f.y.abs());

    if f.x <= f.y {
        DVec2::X
    } else {
        DVec2::Y
    }
}

/// Returns true if left and right are equal within the provided epsilon.
/// Maps to CesiumJS `Cartesian2.equalsEpsilon`
pub fn equals_epsilon(
    left: DVec2,
    right: DVec2,
    relative_epsilon: f64,
    absolute_epsilon: f64,
) -> bool {
    math_utils::equals_epsilon(left.x, right.x, relative_epsilon, absolute_epsilon)
        && math_utils::equals_epsilon(left.y, right.y, relative_epsilon, absolute_epsilon)
}

/// Constrains each component to the given min/max range.
/// Maps to CesiumJS `Cartesian2.clamp`
pub fn clamp(value: DVec2, min: DVec2, max: DVec2) -> DVec2 {
    DVec2::new(
        math_utils::clamp(value.x, min.x, max.x),
        math_utils::clamp(value.y, min.y, max.y),
    )
}

/// Computes a new Cartesian2 with each component set to the absolute value.
/// Maps to CesiumJS `Cartesian2.abs`
pub fn abs(cartesian: DVec2) -> DVec2 {
    DVec2::new(cartesian.x.abs(), cartesian.y.abs())
}

/// Computes the componentwise product of two Cartesians.
/// Maps to CesiumJS `Cartesian2.multiplyComponents`
pub fn multiply_components(left: DVec2, right: DVec2) -> DVec2 {
    DVec2::new(left.x * right.x, left.y * right.y)
}

/// Computes the componentwise quotient of two Cartesians.
/// Maps to CesiumJS `Cartesian2.divideComponents`
pub fn divide_components(left: DVec2, right: DVec2) -> DVec2 {
    DVec2::new(left.x / right.x, left.y / right.y)
}
