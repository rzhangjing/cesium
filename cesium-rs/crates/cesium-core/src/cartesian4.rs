//! Ported from packages/engine/Source/Core/Cartesian4.js
//!
//! A 4D Cartesian point.

use std::fmt;

use crate::check;
use crate::developer_error::throw_developer_error;
use crate::math::CesiumMath;

/// A 4D Cartesian point.
///
/// Port of `Cartesian4`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cartesian4 {
    /// The X component.
    pub x: f64,
    /// The Y component.
    pub y: f64,
    /// The Z component.
    pub z: f64,
    /// The W component.
    pub w: f64,
}

impl Cartesian4 {
    /// The number of elements used to pack the object into an array.
    ///
    /// Port of `Cartesian4.packedLength`.
    pub const PACKED_LENGTH: usize = 4;

    /// An immutable Cartesian4 instance initialized to
    /// (0.0, 0.0, 0.0, 0.0).
    pub const ZERO: Cartesian4 = Cartesian4::new(0.0, 0.0, 0.0, 0.0);

    /// An immutable Cartesian4 instance initialized to
    /// (1.0, 1.0, 1.0, 1.0).
    pub const ONE: Cartesian4 = Cartesian4::new(1.0, 1.0, 1.0, 1.0);

    /// An immutable Cartesian4 instance initialized to
    /// (1.0, 0.0, 0.0, 0.0).
    pub const UNIT_X: Cartesian4 = Cartesian4::new(1.0, 0.0, 0.0, 0.0);

    /// An immutable Cartesian4 instance initialized to
    /// (0.0, 1.0, 0.0, 0.0).
    pub const UNIT_Y: Cartesian4 = Cartesian4::new(0.0, 1.0, 0.0, 0.0);

    /// An immutable Cartesian4 instance initialized to
    /// (0.0, 0.0, 1.0, 0.0).
    pub const UNIT_Z: Cartesian4 = Cartesian4::new(0.0, 0.0, 1.0, 0.0);

    /// An immutable Cartesian4 instance initialized to
    /// (0.0, 0.0, 0.0, 1.0).
    pub const UNIT_W: Cartesian4 = Cartesian4::new(0.0, 0.0, 0.0, 1.0);

    /// Creates a new `Cartesian4`.
    ///
    /// Port of the `Cartesian4(x, y, z, w)` constructor. JS defaults all
    /// components to `0.0` (see `Default`); `new Cartesian4()` maps to
    /// `Cartesian4::default()`.
    pub const fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { x, y, z, w }
    }

    /// Creates a Cartesian4 instance from x, y, z and w coordinates.
    ///
    /// Port of `Cartesian4.fromElements`.
    pub fn from_elements(x: f64, y: f64, z: f64, w: f64, result: &mut Self) {
        result.x = x;
        result.y = y;
        result.z = z;
        result.w = w;
    }

    /// Allocating variant of [`Cartesian4::from_elements`].
    pub fn from_elements_new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self::new(x, y, z, w)
    }

    /// DEVIATION (deferred): `Cartesian4.fromColor` is not ported yet —
    /// it depends on `Color` (red/green/blue/alpha map to x/y/z/w),
    /// which is outside the current batch. Registered in
    /// `docs/deferred.md`; will be added once `Color` is ported.

    /// Duplicates a Cartesian4 instance into `result`.
    ///
    /// Port of `Cartesian4.clone`. The JS `undefined` input case is
    /// statically impossible in Rust; the prototype `clone` maps to the
    /// derived `Clone` trait.
    pub fn clone_into(cartesian: &Self, result: &mut Self) {
        result.x = cartesian.x;
        result.y = cartesian.y;
        result.z = cartesian.z;
        result.w = cartesian.w;
    }

    /// Stores the provided instance into the provided array.
    ///
    /// Port of `Cartesian4.pack`. Returns nothing; the caller already
    /// owns `array` (JS returns the same array reference).
    pub fn pack(value: &Self, array: &mut [f64], starting_index: Option<usize>) {
        let starting_index = starting_index.unwrap_or(0);
        array[starting_index] = value.x;
        array[starting_index + 1] = value.y;
        array[starting_index + 2] = value.z;
        array[starting_index + 3] = value.w;
    }

    /// Retrieves an instance from a packed array.
    ///
    /// Port of `Cartesian4.unpack`.
    pub fn unpack(array: &[f64], starting_index: Option<usize>, result: &mut Self) {
        let starting_index = starting_index.unwrap_or(0);
        result.x = array[starting_index];
        result.y = array[starting_index + 1];
        result.z = array[starting_index + 2];
        result.w = array[starting_index + 3];
    }

    /// Allocating variant of [`Cartesian4::unpack`].
    pub fn unpack_new(array: &[f64], starting_index: Option<usize>) -> Self {
        let mut result = Self::default();
        Self::unpack(array, starting_index, &mut result);
        result
    }

    /// Creates a Cartesian4 from four consecutive elements in an array.
    ///
    /// Port of `Cartesian4.fromArray` (aliased to `Cartesian4.unpack`).
    pub fn from_array(array: &[f64], starting_index: Option<usize>, result: &mut Self) {
        Self::unpack(array, starting_index, result);
    }

    /// Allocating variant of [`Cartesian4::from_array`].
    pub fn from_array_new(array: &[f64], starting_index: Option<usize>) -> Self {
        Self::unpack_new(array, starting_index)
    }

    /// Flattens an array of Cartesian4s into an array of components.
    ///
    /// Port of `Cartesian4.packArray`.
    ///
    /// DEVIATION: JS distinguishes regular arrays (resized) from typed
    /// arrays (must have exactly `array.length * 4` elements, else
    /// `DeveloperError`). A Rust `Vec<f64>` maps to the resizable
    /// regular-array branch; the typed-array exact-length error cannot
    /// be expressed and its spec case is `#[ignore]`d.
    pub fn pack_array(array: &[Self], result: Option<Vec<f64>>) -> Vec<f64> {
        let length = array.len();
        let result_length = length * 4;
        let mut result = match result {
            Some(result) => result,
            None => vec![0.0; result_length],
        };
        result.resize(result_length, 0.0);

        for (i, value) in array.iter().enumerate() {
            Self::pack(value, &mut result, Some(i * 4));
        }

        result
    }

    /// Unpacks an array of cartesian components into an array of
    /// Cartesian4s.
    ///
    /// Port of `Cartesian4.unpackArray`.
    pub fn unpack_array(array: &[f64], result: Option<Vec<Self>>) -> Vec<Self> {
        if cfg!(debug_assertions) {
            check::type_of::number_greater_than_or_equals(
                "array.length",
                array.len() as f64,
                4.0,
            );
            if array.len() % 4 != 0 {
                throw_developer_error("array length must be a multiple of 4.");
            }
        }

        let length = array.len();
        let mut result = match result {
            Some(result) => result,
            None => vec![Self::default(); length / 4],
        };
        result.resize(length / 4, Self::default());

        let mut i = 0;
        while i < length {
            let index = i / 4;
            Self::unpack(array, Some(i), &mut result[index]);
            i += 4;
        }
        result
    }

    /// Computes the value of the maximum component for the supplied
    /// Cartesian.
    ///
    /// Port of `Cartesian4.maximumComponent`.
    pub fn maximum_component(cartesian: &Self) -> f64 {
        cartesian.x.max(cartesian.y).max(cartesian.z).max(cartesian.w)
    }

    /// Computes the value of the minimum component for the supplied
    /// Cartesian.
    ///
    /// Port of `Cartesian4.minimumComponent`.
    pub fn minimum_component(cartesian: &Self) -> f64 {
        cartesian.x.min(cartesian.y).min(cartesian.z).min(cartesian.w)
    }

    /// Compares two Cartesians and computes a Cartesian which contains
    /// the minimum components of the supplied Cartesians.
    ///
    /// Port of `Cartesian4.minimumByComponent`.
    pub fn minimum_by_component(first: &Self, second: &Self, result: &mut Self) {
        result.x = first.x.min(second.x);
        result.y = first.y.min(second.y);
        result.z = first.z.min(second.z);
        result.w = first.w.min(second.w);
    }

    /// Allocating variant of [`Cartesian4::minimum_by_component`].
    pub fn minimum_by_component_new(first: &Self, second: &Self) -> Self {
        let mut result = Self::default();
        Self::minimum_by_component(first, second, &mut result);
        result
    }

    /// Compares two Cartesians and computes a Cartesian which contains
    /// the maximum components of the supplied Cartesians.
    ///
    /// Port of `Cartesian4.maximumByComponent`.
    pub fn maximum_by_component(first: &Self, second: &Self, result: &mut Self) {
        result.x = first.x.max(second.x);
        result.y = first.y.max(second.y);
        result.z = first.z.max(second.z);
        result.w = first.w.max(second.w);
    }

    /// Allocating variant of [`Cartesian4::maximum_by_component`].
    pub fn maximum_by_component_new(first: &Self, second: &Self) -> Self {
        let mut result = Self::default();
        Self::maximum_by_component(first, second, &mut result);
        result
    }

    /// Constrain a value to lie between two values.
    ///
    /// Port of `Cartesian4.clamp`.
    pub fn clamp(value: &Self, min: &Self, max: &Self, result: &mut Self) {
        result.x = CesiumMath::clamp(value.x, min.x, max.x);
        result.y = CesiumMath::clamp(value.y, min.y, max.y);
        result.z = CesiumMath::clamp(value.z, min.z, max.z);
        result.w = CesiumMath::clamp(value.w, min.w, max.w);
    }

    /// Allocating variant of [`Cartesian4::clamp`].
    pub fn clamp_new(value: &Self, min: &Self, max: &Self) -> Self {
        let mut result = Self::default();
        Self::clamp(value, min, max, &mut result);
        result
    }

    /// Computes the provided Cartesian's squared magnitude.
    ///
    /// Port of `Cartesian4.magnitudeSquared`.
    pub fn magnitude_squared(cartesian: &Self) -> f64 {
        cartesian.x * cartesian.x
            + cartesian.y * cartesian.y
            + cartesian.z * cartesian.z
            + cartesian.w * cartesian.w
    }

    /// Computes the Cartesian's magnitude (length).
    ///
    /// Port of `Cartesian4.magnitude`.
    pub fn magnitude(cartesian: &Self) -> f64 {
        Self::magnitude_squared(cartesian).sqrt()
    }

    /// Computes the 4-space distance between two points.
    ///
    /// Port of `Cartesian4.distance`.
    pub fn distance(left: &Self, right: &Self) -> f64 {
        let mut distance_scratch = Self::default();
        Self::subtract(left, right, &mut distance_scratch);
        Self::magnitude(&distance_scratch)
    }

    /// Computes the squared distance between two points. Comparing
    /// squared distances using this function is more efficient than
    /// comparing distances using [`Cartesian4::distance`].
    ///
    /// Port of `Cartesian4.distanceSquared`.
    pub fn distance_squared(left: &Self, right: &Self) -> f64 {
        let mut distance_scratch = Self::default();
        Self::subtract(left, right, &mut distance_scratch);
        Self::magnitude_squared(&distance_scratch)
    }

    /// Computes the normalized form of the supplied Cartesian.
    ///
    /// Port of `Cartesian4.normalize`.
    pub fn normalize(cartesian: &Self, result: &mut Self) {
        let magnitude = Self::magnitude(cartesian);

        result.x = cartesian.x / magnitude;
        result.y = cartesian.y / magnitude;
        result.z = cartesian.z / magnitude;
        result.w = cartesian.w / magnitude;

        if cfg!(debug_assertions) {
            if result.x.is_nan() || result.y.is_nan() || result.z.is_nan() || result.w.is_nan() {
                throw_developer_error("normalized result is not a number");
            }
        }
    }

    /// Allocating variant of [`Cartesian4::normalize`].
    pub fn normalize_new(cartesian: &Self) -> Self {
        let mut result = Self::default();
        Self::normalize(cartesian, &mut result);
        result
    }

    /// Computes the dot (scalar) product of two Cartesians.
    ///
    /// Port of `Cartesian4.dot`.
    pub fn dot(left: &Self, right: &Self) -> f64 {
        left.x * right.x + left.y * right.y + left.z * right.z + left.w * right.w
    }

    /// Computes the componentwise product of two Cartesians.
    ///
    /// Port of `Cartesian4.multiplyComponents`.
    pub fn multiply_components(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x * right.x;
        result.y = left.y * right.y;
        result.z = left.z * right.z;
        result.w = left.w * right.w;
    }

    /// Allocating variant of [`Cartesian4::multiply_components`].
    pub fn multiply_components_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::multiply_components(left, right, &mut result);
        result
    }

    /// Computes the componentwise quotient of two Cartesians.
    ///
    /// Port of `Cartesian4.divideComponents`.
    pub fn divide_components(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x / right.x;
        result.y = left.y / right.y;
        result.z = left.z / right.z;
        result.w = left.w / right.w;
    }

    /// Allocating variant of [`Cartesian4::divide_components`].
    pub fn divide_components_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::divide_components(left, right, &mut result);
        result
    }

    /// Computes the componentwise sum of two Cartesians.
    ///
    /// Port of `Cartesian4.add`.
    pub fn add(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x + right.x;
        result.y = left.y + right.y;
        result.z = left.z + right.z;
        result.w = left.w + right.w;
    }

    /// Allocating variant of [`Cartesian4::add`].
    pub fn add_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::add(left, right, &mut result);
        result
    }

    /// Computes the componentwise difference of two Cartesians.
    ///
    /// Port of `Cartesian4.subtract`.
    pub fn subtract(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x - right.x;
        result.y = left.y - right.y;
        result.z = left.z - right.z;
        result.w = left.w - right.w;
    }

    /// Allocating variant of [`Cartesian4::subtract`].
    pub fn subtract_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::subtract(left, right, &mut result);
        result
    }

    /// Multiplies the provided Cartesian componentwise by the provided
    /// scalar.
    ///
    /// Port of `Cartesian4.multiplyByScalar`.
    pub fn multiply_by_scalar(cartesian: &Self, scalar: f64, result: &mut Self) {
        result.x = cartesian.x * scalar;
        result.y = cartesian.y * scalar;
        result.z = cartesian.z * scalar;
        result.w = cartesian.w * scalar;
    }

    /// Allocating variant of [`Cartesian4::multiply_by_scalar`].
    pub fn multiply_by_scalar_new(cartesian: &Self, scalar: f64) -> Self {
        let mut result = Self::default();
        Self::multiply_by_scalar(cartesian, scalar, &mut result);
        result
    }

    /// Divides the provided Cartesian componentwise by the provided
    /// scalar.
    ///
    /// Port of `Cartesian4.divideByScalar`.
    pub fn divide_by_scalar(cartesian: &Self, scalar: f64, result: &mut Self) {
        result.x = cartesian.x / scalar;
        result.y = cartesian.y / scalar;
        result.z = cartesian.z / scalar;
        result.w = cartesian.w / scalar;
    }

    /// Allocating variant of [`Cartesian4::divide_by_scalar`].
    pub fn divide_by_scalar_new(cartesian: &Self, scalar: f64) -> Self {
        let mut result = Self::default();
        Self::divide_by_scalar(cartesian, scalar, &mut result);
        result
    }

    /// Negates the provided Cartesian.
    ///
    /// Port of `Cartesian4.negate`.
    pub fn negate(cartesian: &Self, result: &mut Self) {
        result.x = -cartesian.x;
        result.y = -cartesian.y;
        result.z = -cartesian.z;
        result.w = -cartesian.w;
    }

    /// Allocating variant of [`Cartesian4::negate`].
    pub fn negate_new(cartesian: &Self) -> Self {
        let mut result = Self::default();
        Self::negate(cartesian, &mut result);
        result
    }

    /// Computes the absolute value of the provided Cartesian.
    ///
    /// Port of `Cartesian4.abs`.
    pub fn abs(cartesian: &Self, result: &mut Self) {
        result.x = cartesian.x.abs();
        result.y = cartesian.y.abs();
        result.z = cartesian.z.abs();
        result.w = cartesian.w.abs();
    }

    /// Allocating variant of [`Cartesian4::abs`].
    pub fn abs_new(cartesian: &Self) -> Self {
        let mut result = Self::default();
        Self::abs(cartesian, &mut result);
        result
    }

    /// Computes the linear interpolation or extrapolation at t using the
    /// provided cartesians.
    ///
    /// Port of `Cartesian4.lerp`.
    pub fn lerp(start: &Self, end: &Self, t: f64, result: &mut Self) {
        let mut lerp_scratch = Self::default();
        Self::multiply_by_scalar(end, t, &mut lerp_scratch);
        Self::multiply_by_scalar(start, 1.0 - t, result);
        // `result` aliases itself as input; copy first (values are Copy).
        let current = *result;
        Self::add(&lerp_scratch, &current, result);
    }

    /// Allocating variant of [`Cartesian4::lerp`].
    pub fn lerp_new(start: &Self, end: &Self, t: f64) -> Self {
        let mut result = Self::default();
        Self::lerp(start, end, t, &mut result);
        result
    }

    /// Returns the axis that is most orthogonal to the provided
    /// Cartesian.
    ///
    /// Port of `Cartesian4.mostOrthogonalAxis`.
    pub fn most_orthogonal_axis(cartesian: &Self, result: &mut Self) {
        let mut most_orthogonal_axis_scratch = Self::default();
        Self::normalize(cartesian, &mut most_orthogonal_axis_scratch);
        let normalized = most_orthogonal_axis_scratch;
        Self::abs(&normalized, &mut most_orthogonal_axis_scratch);
        let f = most_orthogonal_axis_scratch;

        let unit = if f.x <= f.y {
            if f.x <= f.z {
                if f.x <= f.w {
                    Self::UNIT_X
                } else {
                    Self::UNIT_W
                }
            } else if f.z <= f.w {
                Self::UNIT_Z
            } else {
                Self::UNIT_W
            }
        } else if f.y <= f.z {
            if f.y <= f.w {
                Self::UNIT_Y
            } else {
                Self::UNIT_W
            }
        } else if f.z <= f.w {
            Self::UNIT_Z
        } else {
            Self::UNIT_W
        };
        Self::clone_into(&unit, result);
    }

    /// Compares the provided Cartesians componentwise and returns true
    /// if they are equal, false otherwise.
    ///
    /// Port of `Cartesian4.equals`. `None` mirrors JS `undefined`.
    pub fn equals(left: Option<&Self>, right: Option<&Self>) -> bool {
        match (left, right) {
            (Some(left), Some(right)) => {
                left.x == right.x
                    && left.y == right.y
                    && left.z == right.z
                    && left.w == right.w
            }
            (None, None) => true,
            _ => false,
        }
    }

    /// Port of `Cartesian4.equalsArray` (`@ignore` in JS).
    pub fn equals_array(cartesian: &Self, array: &[f64], offset: usize) -> bool {
        cartesian.x == array[offset]
            && cartesian.y == array[offset + 1]
            && cartesian.z == array[offset + 2]
            && cartesian.w == array[offset + 3]
    }

    /// Compares the provided Cartesians componentwise and returns true
    /// if they pass an absolute or relative tolerance test, false
    /// otherwise.
    ///
    /// Port of `Cartesian4.equalsEpsilon`. `None` mirrors JS
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
                    && CesiumMath::equals_epsilon(
                        left.z,
                        right.z,
                        relative_epsilon,
                        absolute_epsilon,
                    )
                    && CesiumMath::equals_epsilon(
                        left.w,
                        right.w,
                        relative_epsilon,
                        absolute_epsilon,
                    )
            }
            (None, None) => true,
            _ => false,
        }
    }

    /// Packs an arbitrary floating point value to 4 values representable
    /// using uint8.
    ///
    /// Port of `Cartesian4.packFloat`. The JS version stores the value
    /// into a `Float32Array` and reads the 4 bytes through a
    /// `Uint8Array` view over the same buffer, normalizing to
    /// little-endian order. Rust maps this to `f32::to_le_bytes`.
    pub fn pack_float(value: f64, result: &mut Self) {
        let bytes = (value as f32).to_le_bytes();
        result.x = bytes[0] as f64;
        result.y = bytes[1] as f64;
        result.z = bytes[2] as f64;
        result.w = bytes[3] as f64;
    }

    /// Allocating variant of [`Cartesian4::pack_float`].
    pub fn pack_float_new(value: f64) -> Self {
        let mut result = Self::default();
        Self::pack_float(value, &mut result);
        result
    }

    /// Unpacks a float packed using [`Cartesian4::pack_float`].
    ///
    /// Port of `Cartesian4.unpackFloat` (`@private` in JS). Component
    /// assignment into a `Uint8Array` uses JS `ToUint8` (modulo 2^8)
    /// semantics, mirrored by `rem_euclid(256.0)` below.
    pub fn unpack_float(packed_float: &Self) -> f64 {
        let bytes = [
            to_uint8(packed_float.x),
            to_uint8(packed_float.y),
            to_uint8(packed_float.z),
            to_uint8(packed_float.w),
        ];
        f32::from_le_bytes(bytes) as f64
    }

    /// Compares this Cartesian against the provided Cartesian
    /// componentwise and returns true if they are equal.
    ///
    /// Port of `Cartesian4.prototype.equals`.
    pub fn equals_method(&self, right: &Self) -> bool {
        Self::equals(Some(self), Some(right))
    }

    /// Compares this Cartesian against the provided Cartesian
    /// componentwise and returns true if they pass an absolute or
    /// relative tolerance test.
    ///
    /// Port of `Cartesian4.prototype.equalsEpsilon`.
    pub fn equals_epsilon_method(
        &self,
        right: &Self,
        relative_epsilon: Option<f64>,
        absolute_epsilon: Option<f64>,
    ) -> bool {
        Self::equals_epsilon(Some(self), Some(right), relative_epsilon, absolute_epsilon)
    }
}

/// JS `ToUint8` conversion used when writing into a `Uint8Array`
/// (truncate toward zero, then modulo 2^8; `NaN`/infinity → 0).
fn to_uint8(value: f64) -> u8 {
    if !value.is_finite() {
        return 0;
    }
    value.trunc().rem_euclid(256.0) as u8
}

impl Default for Cartesian4 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl fmt::Display for Cartesian4 {
    /// Port of `Cartesian4.prototype.toString` — format `(x, y, z, w)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}
