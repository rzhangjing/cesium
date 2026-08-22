//! Ported from packages/engine/Source/Core/Cartesian2.js
//!
//! A 2D Cartesian point.

use std::fmt;

use crate::cartesian3::Cartesian3;
use crate::cartesian4::Cartesian4;
use crate::check;
use crate::developer_error::throw_developer_error;
use crate::math::CesiumMath;

/// A 2D Cartesian point.
///
/// Port of `Cartesian2`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cartesian2 {
    /// The X component.
    pub x: f64,
    /// The Y component.
    pub y: f64,
}

impl Cartesian2 {
    /// The number of elements used to pack the object into an array.
    ///
    /// Port of `Cartesian2.packedLength`.
    pub const PACKED_LENGTH: usize = 2;

    /// An immutable Cartesian2 instance initialized to (0.0, 0.0).
    pub const ZERO: Cartesian2 = Cartesian2::new(0.0, 0.0);

    /// An immutable Cartesian2 instance initialized to (1.0, 1.0).
    pub const ONE: Cartesian2 = Cartesian2::new(1.0, 1.0);

    /// An immutable Cartesian2 instance initialized to (1.0, 0.0).
    pub const UNIT_X: Cartesian2 = Cartesian2::new(1.0, 0.0);

    /// An immutable Cartesian2 instance initialized to (0.0, 1.0).
    pub const UNIT_Y: Cartesian2 = Cartesian2::new(0.0, 1.0);

    /// Creates a new `Cartesian2`.
    ///
    /// Port of the `Cartesian2(x, y)` constructor. JS defaults both
    /// components to `0.0` (see `Default`); `new Cartesian2()` maps to
    /// `Cartesian2::default()`.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Creates a Cartesian2 instance from x and y coordinates.
    ///
    /// Port of `Cartesian2.fromElements`.
    pub fn from_elements(x: f64, y: f64, result: &mut Self) {
        result.x = x;
        result.y = y;
    }

    /// Allocating variant of [`Cartesian2::from_elements`].
    pub fn from_elements_new(x: f64, y: f64) -> Self {
        Self::new(x, y)
    }

    /// Duplicates a Cartesian2 instance into `result`.
    ///
    /// Port of `Cartesian2.clone`. The JS `undefined` input case is
    /// statically impossible in Rust; the prototype `clone` maps to the
    /// derived `Clone` trait.
    pub fn clone_into(cartesian: &Self, result: &mut Self) {
        result.x = cartesian.x;
        result.y = cartesian.y;
    }

    /// Creates a Cartesian2 instance from an existing Cartesian3. This
    /// simply takes the x and y properties of the Cartesian3 and drops
    /// z.
    ///
    /// Port of `Cartesian2.fromCartesian3` (aliased to
    /// `Cartesian2.clone` in JS via duck typing; made explicit in
    /// Rust).
    pub fn from_cartesian3(cartesian: &Cartesian3, result: &mut Self) {
        result.x = cartesian.x;
        result.y = cartesian.y;
    }

    /// Allocating variant of [`Cartesian2::from_cartesian3`].
    pub fn from_cartesian3_new(cartesian: &Cartesian3) -> Self {
        Self::new(cartesian.x, cartesian.y)
    }

    /// Creates a Cartesian2 instance from an existing Cartesian4. This
    /// simply takes the x and y properties of the Cartesian4 and drops
    /// z and w.
    ///
    /// Port of `Cartesian2.fromCartesian4` (aliased to
    /// `Cartesian2.clone` in JS via duck typing; made explicit in
    /// Rust).
    pub fn from_cartesian4(cartesian: &Cartesian4, result: &mut Self) {
        result.x = cartesian.x;
        result.y = cartesian.y;
    }

    /// Allocating variant of [`Cartesian2::from_cartesian4`].
    pub fn from_cartesian4_new(cartesian: &Cartesian4) -> Self {
        Self::new(cartesian.x, cartesian.y)
    }

    /// Stores the provided instance into the provided array.
    ///
    /// Port of `Cartesian2.pack`. Returns nothing; the caller already
    /// owns `array` (JS returns the same array reference).
    pub fn pack(value: &Self, array: &mut [f64], starting_index: Option<usize>) {
        let starting_index = starting_index.unwrap_or(0);
        array[starting_index] = value.x;
        array[starting_index + 1] = value.y;
    }

    /// Retrieves an instance from a packed array.
    ///
    /// Port of `Cartesian2.unpack`.
    pub fn unpack(array: &[f64], starting_index: Option<usize>, result: &mut Self) {
        let starting_index = starting_index.unwrap_or(0);
        result.x = array[starting_index];
        result.y = array[starting_index + 1];
    }

    /// Allocating variant of [`Cartesian2::unpack`].
    pub fn unpack_new(array: &[f64], starting_index: Option<usize>) -> Self {
        let mut result = Self::default();
        Self::unpack(array, starting_index, &mut result);
        result
    }

    /// Creates a Cartesian2 from two consecutive elements in an array.
    ///
    /// Port of `Cartesian2.fromArray` (aliased to `Cartesian2.unpack`).
    pub fn from_array(array: &[f64], starting_index: Option<usize>, result: &mut Self) {
        Self::unpack(array, starting_index, result);
    }

    /// Allocating variant of [`Cartesian2::from_array`].
    pub fn from_array_new(array: &[f64], starting_index: Option<usize>) -> Self {
        Self::unpack_new(array, starting_index)
    }

    /// Flattens an array of Cartesian2s into an array of components.
    ///
    /// Port of `Cartesian2.packArray`.
    ///
    /// DEVIATION: JS distinguishes regular arrays (resized) from typed
    /// arrays (must have exactly `array.length * 2` elements, else
    /// `DeveloperError`). A Rust `Vec<f64>` maps to the resizable
    /// regular-array branch; the typed-array exact-length error cannot
    /// be expressed and its spec case is `#[ignore]`d.
    pub fn pack_array(array: &[Self], result: Option<Vec<f64>>) -> Vec<f64> {
        let length = array.len();
        let result_length = length * 2;
        let mut result = match result {
            Some(result) => result,
            None => vec![0.0; result_length],
        };
        result.resize(result_length, 0.0);

        for (i, value) in array.iter().enumerate() {
            Self::pack(value, &mut result, Some(i * 2));
        }

        result
    }

    /// Unpacks an array of cartesian components into an array of
    /// Cartesian2s.
    ///
    /// Port of `Cartesian2.unpackArray`.
    pub fn unpack_array(array: &[f64], result: Option<Vec<Self>>) -> Vec<Self> {
        if cfg!(debug_assertions) {
            check::type_of::number_greater_than_or_equals(
                "array.length",
                array.len() as f64,
                2.0,
            );
            if array.len() % 2 != 0 {
                throw_developer_error("array length must be a multiple of 2.");
            }
        }

        let length = array.len();
        let mut result = match result {
            Some(result) => result,
            None => vec![Self::default(); length / 2],
        };
        result.resize(length / 2, Self::default());

        let mut i = 0;
        while i < length {
            let index = i / 2;
            Self::unpack(array, Some(i), &mut result[index]);
            i += 2;
        }
        result
    }

    /// Computes the value of the maximum component for the supplied
    /// Cartesian.
    ///
    /// Port of `Cartesian2.maximumComponent`.
    pub fn maximum_component(cartesian: &Self) -> f64 {
        cartesian.x.max(cartesian.y)
    }

    /// Computes the value of the minimum component for the supplied
    /// Cartesian.
    ///
    /// Port of `Cartesian2.minimumComponent`.
    pub fn minimum_component(cartesian: &Self) -> f64 {
        cartesian.x.min(cartesian.y)
    }

    /// Compares two Cartesians and computes a Cartesian which contains
    /// the minimum components of the supplied Cartesians.
    ///
    /// Port of `Cartesian2.minimumByComponent`.
    pub fn minimum_by_component(first: &Self, second: &Self, result: &mut Self) {
        result.x = first.x.min(second.x);
        result.y = first.y.min(second.y);
    }

    /// Allocating variant of [`Cartesian2::minimum_by_component`].
    pub fn minimum_by_component_new(first: &Self, second: &Self) -> Self {
        let mut result = Self::default();
        Self::minimum_by_component(first, second, &mut result);
        result
    }

    /// Compares two Cartesians and computes a Cartesian which contains
    /// the maximum components of the supplied Cartesians.
    ///
    /// Port of `Cartesian2.maximumByComponent`.
    pub fn maximum_by_component(first: &Self, second: &Self, result: &mut Self) {
        result.x = first.x.max(second.x);
        result.y = first.y.max(second.y);
    }

    /// Allocating variant of [`Cartesian2::maximum_by_component`].
    pub fn maximum_by_component_new(first: &Self, second: &Self) -> Self {
        let mut result = Self::default();
        Self::maximum_by_component(first, second, &mut result);
        result
    }

    /// Constrain a value to lie between two values.
    ///
    /// Port of `Cartesian2.clamp`.
    pub fn clamp(value: &Self, min: &Self, max: &Self, result: &mut Self) {
        result.x = CesiumMath::clamp(value.x, min.x, max.x);
        result.y = CesiumMath::clamp(value.y, min.y, max.y);
    }

    /// Allocating variant of [`Cartesian2::clamp`].
    pub fn clamp_new(value: &Self, min: &Self, max: &Self) -> Self {
        let mut result = Self::default();
        Self::clamp(value, min, max, &mut result);
        result
    }

    /// Computes the provided Cartesian's squared magnitude.
    ///
    /// Port of `Cartesian2.magnitudeSquared`.
    pub fn magnitude_squared(cartesian: &Self) -> f64 {
        cartesian.x * cartesian.x + cartesian.y * cartesian.y
    }

    /// Computes the Cartesian's magnitude (length).
    ///
    /// Port of `Cartesian2.magnitude`.
    pub fn magnitude(cartesian: &Self) -> f64 {
        Self::magnitude_squared(cartesian).sqrt()
    }

    /// Computes the distance between two points.
    ///
    /// Port of `Cartesian2.distance`.
    pub fn distance(left: &Self, right: &Self) -> f64 {
        let mut distance_scratch = Self::default();
        Self::subtract(left, right, &mut distance_scratch);
        Self::magnitude(&distance_scratch)
    }

    /// Computes the squared distance between two points. Comparing
    /// squared distances using this function is more efficient than
    /// comparing distances using [`Cartesian2::distance`].
    ///
    /// Port of `Cartesian2.distanceSquared`.
    pub fn distance_squared(left: &Self, right: &Self) -> f64 {
        let mut distance_scratch = Self::default();
        Self::subtract(left, right, &mut distance_scratch);
        Self::magnitude_squared(&distance_scratch)
    }

    /// Computes the normalized form of the supplied Cartesian.
    ///
    /// Port of `Cartesian2.normalize`.
    pub fn normalize(cartesian: &Self, result: &mut Self) {
        let magnitude = Self::magnitude(cartesian);

        result.x = cartesian.x / magnitude;
        result.y = cartesian.y / magnitude;

        if cfg!(debug_assertions) {
            if result.x.is_nan() || result.y.is_nan() {
                throw_developer_error("normalized result is not a number");
            }
        }
    }

    /// Allocating variant of [`Cartesian2::normalize`].
    pub fn normalize_new(cartesian: &Self) -> Self {
        let mut result = Self::default();
        Self::normalize(cartesian, &mut result);
        result
    }

    /// Computes the dot (scalar) product of two Cartesians.
    ///
    /// Port of `Cartesian2.dot`.
    pub fn dot(left: &Self, right: &Self) -> f64 {
        left.x * right.x + left.y * right.y
    }

    /// Computes the magnitude of the cross product that would result
    /// from implicitly setting the Z coordinate of the input vectors to
    /// 0.
    ///
    /// Port of `Cartesian2.cross`. Returns the scalar cross value
    /// (unlike `Cartesian3.cross`).
    pub fn cross(left: &Self, right: &Self) -> f64 {
        left.x * right.y - left.y * right.x
    }

    /// Computes the componentwise product of two Cartesians.
    ///
    /// Port of `Cartesian2.multiplyComponents`.
    pub fn multiply_components(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x * right.x;
        result.y = left.y * right.y;
    }

    /// Allocating variant of [`Cartesian2::multiply_components`].
    pub fn multiply_components_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::multiply_components(left, right, &mut result);
        result
    }

    /// Computes the componentwise quotient of two Cartesians.
    ///
    /// Port of `Cartesian2.divideComponents`.
    pub fn divide_components(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x / right.x;
        result.y = left.y / right.y;
    }

    /// Allocating variant of [`Cartesian2::divide_components`].
    pub fn divide_components_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::divide_components(left, right, &mut result);
        result
    }

    /// Computes the componentwise sum of two Cartesians.
    ///
    /// Port of `Cartesian2.add`.
    pub fn add(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x + right.x;
        result.y = left.y + right.y;
    }

    /// Allocating variant of [`Cartesian2::add`].
    pub fn add_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::add(left, right, &mut result);
        result
    }

    /// Computes the componentwise difference of two Cartesians.
    ///
    /// Port of `Cartesian2.subtract`.
    pub fn subtract(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x - right.x;
        result.y = left.y - right.y;
    }

    /// Allocating variant of [`Cartesian2::subtract`].
    pub fn subtract_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::subtract(left, right, &mut result);
        result
    }

    /// Multiplies the provided Cartesian componentwise by the provided
    /// scalar.
    ///
    /// Port of `Cartesian2.multiplyByScalar`.
    pub fn multiply_by_scalar(cartesian: &Self, scalar: f64, result: &mut Self) {
        result.x = cartesian.x * scalar;
        result.y = cartesian.y * scalar;
    }

    /// Allocating variant of [`Cartesian2::multiply_by_scalar`].
    pub fn multiply_by_scalar_new(cartesian: &Self, scalar: f64) -> Self {
        let mut result = Self::default();
        Self::multiply_by_scalar(cartesian, scalar, &mut result);
        result
    }

    /// Divides the provided Cartesian componentwise by the provided
    /// scalar.
    ///
    /// Port of `Cartesian2.divideByScalar`.
    pub fn divide_by_scalar(cartesian: &Self, scalar: f64, result: &mut Self) {
        result.x = cartesian.x / scalar;
        result.y = cartesian.y / scalar;
    }

    /// Allocating variant of [`Cartesian2::divide_by_scalar`].
    pub fn divide_by_scalar_new(cartesian: &Self, scalar: f64) -> Self {
        let mut result = Self::default();
        Self::divide_by_scalar(cartesian, scalar, &mut result);
        result
    }

    /// Negates the provided Cartesian.
    ///
    /// Port of `Cartesian2.negate`.
    pub fn negate(cartesian: &Self, result: &mut Self) {
        result.x = -cartesian.x;
        result.y = -cartesian.y;
    }

    /// Allocating variant of [`Cartesian2::negate`].
    pub fn negate_new(cartesian: &Self) -> Self {
        let mut result = Self::default();
        Self::negate(cartesian, &mut result);
        result
    }

    /// Computes the absolute value of the provided Cartesian.
    ///
    /// Port of `Cartesian2.abs`.
    pub fn abs(cartesian: &Self, result: &mut Self) {
        result.x = cartesian.x.abs();
        result.y = cartesian.y.abs();
    }

    /// Allocating variant of [`Cartesian2::abs`].
    pub fn abs_new(cartesian: &Self) -> Self {
        let mut result = Self::default();
        Self::abs(cartesian, &mut result);
        result
    }

    /// Computes the linear interpolation or extrapolation at t using the
    /// provided cartesians.
    ///
    /// Port of `Cartesian2.lerp`.
    pub fn lerp(start: &Self, end: &Self, t: f64, result: &mut Self) {
        let mut lerp_scratch = Self::default();
        Self::multiply_by_scalar(end, t, &mut lerp_scratch);
        Self::multiply_by_scalar(start, 1.0 - t, result);
        // `result` aliases itself as input; copy first (values are Copy).
        let current = *result;
        Self::add(&lerp_scratch, &current, result);
    }

    /// Allocating variant of [`Cartesian2::lerp`].
    pub fn lerp_new(start: &Self, end: &Self, t: f64) -> Self {
        let mut result = Self::default();
        Self::lerp(start, end, t, &mut result);
        result
    }

    /// Returns the angle, in radians, between the provided Cartesians.
    ///
    /// Port of `Cartesian2.angleBetween`. Note: unlike
    /// `Cartesian3.angleBetween` this uses `CesiumMath.acosClamped` on
    /// the dot product of the normalized vectors.
    pub fn angle_between(left: &Self, right: &Self) -> f64 {
        let mut angle_between_scratch = Self::default();
        let mut angle_between_scratch2 = Self::default();
        Self::normalize(left, &mut angle_between_scratch);
        Self::normalize(right, &mut angle_between_scratch2);
        CesiumMath::acos_clamped(Self::dot(&angle_between_scratch, &angle_between_scratch2))
    }

    /// Returns the axis that is most orthogonal to the provided
    /// Cartesian.
    ///
    /// Port of `Cartesian2.mostOrthogonalAxis`.
    pub fn most_orthogonal_axis(cartesian: &Self, result: &mut Self) {
        let mut most_orthogonal_axis_scratch = Self::default();
        Self::normalize(cartesian, &mut most_orthogonal_axis_scratch);
        let normalized = most_orthogonal_axis_scratch;
        Self::abs(&normalized, &mut most_orthogonal_axis_scratch);
        let f = most_orthogonal_axis_scratch;

        let unit = if f.x <= f.y { Self::UNIT_X } else { Self::UNIT_Y };
        Self::clone_into(&unit, result);
    }

    /// Compares the provided Cartesians componentwise and returns true
    /// if they are equal, false otherwise.
    ///
    /// Port of `Cartesian2.equals`. `None` mirrors JS `undefined`.
    pub fn equals(left: Option<&Self>, right: Option<&Self>) -> bool {
        match (left, right) {
            (Some(left), Some(right)) => left.x == right.x && left.y == right.y,
            (None, None) => true,
            _ => false,
        }
    }

    /// Port of `Cartesian2.equalsArray` (`@ignore` in JS).
    pub fn equals_array(cartesian: &Self, array: &[f64], offset: usize) -> bool {
        cartesian.x == array[offset] && cartesian.y == array[offset + 1]
    }

    /// Compares the provided Cartesians componentwise and returns true
    /// if they pass an absolute or relative tolerance test, false
    /// otherwise.
    ///
    /// Port of `Cartesian2.equalsEpsilon`. `None` mirrors JS
    /// `undefined`.
    pub fn equals_epsilon(
        left: Option<&Self>,
        right: Option<&Self>,
        relative_epsilon: Option<f64>,
        absolute_epsilon: Option<f64>,
    ) -> bool {
        match (left, right) {
            (Some(left), Some(right)) => {
                CesiumMath::equals_epsilon(left.x, right.x, relative_epsilon, absolute_epsilon)
                    && CesiumMath::equals_epsilon(
                        left.y,
                        right.y,
                        relative_epsilon,
                        absolute_epsilon,
                    )
            }
            (None, None) => true,
            _ => false,
        }
    }

    /// Compares this Cartesian against the provided Cartesian
    /// componentwise and returns true if they are equal.
    ///
    /// Port of `Cartesian2.prototype.equals`.
    pub fn equals_method(&self, right: &Self) -> bool {
        Self::equals(Some(self), Some(right))
    }

    /// Compares this Cartesian against the provided Cartesian
    /// componentwise and returns true if they pass an absolute or
    /// relative tolerance test.
    ///
    /// Port of `Cartesian2.prototype.equalsEpsilon`.
    pub fn equals_epsilon_method(
        &self,
        right: &Self,
        relative_epsilon: Option<f64>,
        absolute_epsilon: Option<f64>,
    ) -> bool {
        Self::equals_epsilon(Some(self), Some(right), relative_epsilon, absolute_epsilon)
    }
}

impl Default for Cartesian2 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl fmt::Display for Cartesian2 {
    /// Port of `Cartesian2.prototype.toString` — format `(x, y)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}
