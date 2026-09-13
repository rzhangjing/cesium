//! Cartesian4 CesiumJS extension functions.
//! Maps to CesiumJS `Core/Cartesian4.js` static methods that go beyond basic vector math.

use crate::math_utils;
use glam::DVec4;

/// The packed length of a Cartesian4: 4.
pub const PACKED_LENGTH: usize = 4;

/// Packs a Cartesian4 into an array at the given starting index.
/// Maps to CesiumJS `Cartesian4.pack`
pub fn pack(value: DVec4, array: &mut [f64], starting_index: usize) {
    array[starting_index] = value.x;
    array[starting_index + 1] = value.y;
    array[starting_index + 2] = value.z;
    array[starting_index + 3] = value.w;
}

/// Unpacks a Cartesian4 from an array at the given starting index.
/// Maps to CesiumJS `Cartesian4.unpack`
pub fn unpack(array: &[f64], starting_index: usize) -> DVec4 {
    DVec4::new(
        array[starting_index],
        array[starting_index + 1],
        array[starting_index + 2],
        array[starting_index + 3],
    )
}

/// Flattens an array of Cartesian4s into an array of components.
/// Maps to CesiumJS `Cartesian4.packArray`
pub fn pack_array(array: &[DVec4]) -> Vec<f64> {
    let length = array.len();
    let mut result = vec![0.0f64; length * 4];
    for i in 0..length {
        pack(array[i], &mut result, i * 4);
    }
    result
}

/// Unpacks an array of components into an array of Cartesian4s.
/// Maps to CesiumJS `Cartesian4.unpackArray`
pub fn unpack_array(array: &[f64]) -> Vec<DVec4> {
    let length = array.len() / 4;
    let mut result = Vec::with_capacity(length);
    for i in 0..length {
        result.push(unpack(array, i * 4));
    }
    result
}

/// Creates a Cartesian4 from the first four elements of an array at an offset.
/// Maps to CesiumJS `Cartesian4.fromArray`
pub fn from_array(array: &[f64], starting_index: usize) -> DVec4 {
    DVec4::new(
        array[starting_index],
        array[starting_index + 1],
        array[starting_index + 2],
        array[starting_index + 3],
    )
}

/// Returns the component with the maximum value.
/// Maps to CesiumJS `Cartesian4.maximumComponent`
pub fn maximum_component(cartesian: DVec4) -> f64 {
    cartesian.x.max(cartesian.y).max(cartesian.z).max(cartesian.w)
}

/// Returns the component with the minimum value.
/// Maps to CesiumJS `Cartesian4.minimumComponent`
pub fn minimum_component(cartesian: DVec4) -> f64 {
    cartesian.x.min(cartesian.y).min(cartesian.z).min(cartesian.w)
}

/// Computes the provided Cartesian's squared magnitude.
/// Maps to CesiumJS `Cartesian4.magnitudeSquared`
pub fn magnitude_squared(cartesian: DVec4) -> f64 {
    cartesian.x * cartesian.x
        + cartesian.y * cartesian.y
        + cartesian.z * cartesian.z
        + cartesian.w * cartesian.w
}

/// Computes the Cartesian's magnitude (length).
/// Maps to CesiumJS `Cartesian4.magnitude`
pub fn magnitude(cartesian: DVec4) -> f64 {
    magnitude_squared(cartesian).sqrt()
}

/// Computes the distance between two points.
/// Maps to CesiumJS `Cartesian4.distance`
pub fn distance(left: DVec4, right: DVec4) -> f64 {
    (left - right).length()
}

/// Computes the squared distance between two points.
/// Maps to CesiumJS `Cartesian4.distanceSquared`
pub fn distance_squared(left: DVec4, right: DVec4) -> f64 {
    (left - right).length_squared()
}

/// Computes the linear interpolation or extrapolation at t using the provided cartesians.
/// Maps to CesiumJS `Cartesian4.lerp`
pub fn lerp(start: DVec4, end: DVec4, t: f64) -> DVec4 {
    start + (end - start) * t
}

/// Computes the angle between two vectors.
/// Maps to CesiumJS `Cartesian4.angleBetween`
pub fn angle_between(left: DVec4, right: DVec4) -> f64 {
    let dot_val = left.dot(right);
    let magnitude_left_sq = left.dot(left);
    let magnitude_right_sq = right.dot(right);
    let cross_magnitude = (magnitude_left_sq * magnitude_right_sq - dot_val * dot_val)
        .max(0.0)
        .sqrt();
    cross_magnitude.atan2(dot_val)
}

/// Returns true if left and right are equal within the provided epsilon.
/// Maps to CesiumJS `Cartesian4.equalsEpsilon`
pub fn equals_epsilon(
    left: DVec4,
    right: DVec4,
    relative_epsilon: f64,
    absolute_epsilon: f64,
) -> bool {
    math_utils::equals_epsilon(left.x, right.x, relative_epsilon, absolute_epsilon)
        && math_utils::equals_epsilon(left.y, right.y, relative_epsilon, absolute_epsilon)
        && math_utils::equals_epsilon(left.z, right.z, relative_epsilon, absolute_epsilon)
        && math_utils::equals_epsilon(left.w, right.w, relative_epsilon, absolute_epsilon)
}

/// Constrains each component to the given min/max range.
/// Maps to CesiumJS `Cartesian4.clamp`
pub fn clamp(value: DVec4, min: DVec4, max: DVec4) -> DVec4 {
    DVec4::new(
        math_utils::clamp(value.x, min.x, max.x),
        math_utils::clamp(value.y, min.y, max.y),
        math_utils::clamp(value.z, min.z, max.z),
        math_utils::clamp(value.w, min.w, max.w),
    )
}

/// Computes a new Cartesian4 with each component set to the absolute value.
/// Maps to CesiumJS `Cartesian4.abs`
pub fn abs(cartesian: DVec4) -> DVec4 {
    DVec4::new(
        cartesian.x.abs(),
        cartesian.y.abs(),
        cartesian.z.abs(),
        cartesian.w.abs(),
    )
}

/// Computes the componentwise product of two Cartesians.
/// Maps to CesiumJS `Cartesian4.multiplyComponents`
pub fn multiply_components(left: DVec4, right: DVec4) -> DVec4 {
    DVec4::new(
        left.x * right.x,
        left.y * right.y,
        left.z * right.z,
        left.w * right.w,
    )
}

/// Computes the componentwise quotient of two Cartesians.
/// Maps to CesiumJS `Cartesian4.divideComponents`
pub fn divide_components(left: DVec4, right: DVec4) -> DVec4 {
    DVec4::new(
        left.x / right.x,
        left.y / right.y,
        left.z / right.z,
        left.w / right.w,
    )
}
