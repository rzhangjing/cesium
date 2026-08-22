//! Ported from packages/engine/Source/Core/Cartesian3.js
//!
//! A 3D Cartesian point.

use std::fmt;
use std::sync::Mutex;

use crate::cartesian4::Cartesian4;
use crate::check;
use crate::developer_error::throw_developer_error;
use crate::math::CesiumMath;
use crate::spherical::Spherical;

/// A 3D Cartesian point.
///
/// Port of `Cartesian3`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cartesian3 {
    /// The X component.
    pub x: f64,
    /// The Y component.
    pub y: f64,
    /// The Z component.
    pub z: f64,
}

impl Cartesian3 {
    /// The number of elements used to pack the object into an array.
    ///
    /// Port of `Cartesian3.packedLength`.
    pub const PACKED_LENGTH: usize = 3;

    /// An immutable Cartesian3 instance initialized to (0.0, 0.0, 0.0).
    pub const ZERO: Cartesian3 = Cartesian3::new(0.0, 0.0, 0.0);

    /// An immutable Cartesian3 instance initialized to (1.0, 1.0, 1.0).
    pub const ONE: Cartesian3 = Cartesian3::new(1.0, 1.0, 1.0);

    /// An immutable Cartesian3 instance initialized to (1.0, 0.0, 0.0).
    pub const UNIT_X: Cartesian3 = Cartesian3::new(1.0, 0.0, 0.0);

    /// An immutable Cartesian3 instance initialized to (0.0, 1.0, 0.0).
    pub const UNIT_Y: Cartesian3 = Cartesian3::new(0.0, 1.0, 0.0);

    /// An immutable Cartesian3 instance initialized to (0.0, 0.0, 1.0).
    pub const UNIT_Z: Cartesian3 = Cartesian3::new(0.0, 0.0, 1.0);

    /// Creates a new `Cartesian3`.
    ///
    /// Port of the `Cartesian3(x, y, z)` constructor. JS defaults all
    /// components to `0.0` (see `Default`); `new Cartesian3()` maps to
    /// `Cartesian3::default()`.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Creates a Cartesian3 instance from a Spherical.
    ///
    /// Port of `Cartesian3.fromSpherical`.
    pub fn from_spherical(spherical: &Spherical, result: &mut Self) {
        let clock = spherical.clock;
        let cone = spherical.cone;
        // JS: `spherical.magnitude ?? 1.0`; Rust field is always defined
        // and `Spherical::default()` already uses 1.0.
        let magnitude = spherical.magnitude;
        let radial = magnitude * cone.sin();
        result.x = radial * clock.cos();
        result.y = radial * clock.sin();
        result.z = magnitude * cone.cos();
    }

    /// Allocating variant of [`Cartesian3::from_spherical`].
    pub fn from_spherical_new(spherical: &Spherical) -> Self {
        let mut result = Self::default();
        Self::from_spherical(spherical, &mut result);
        result
    }

    /// Creates a Cartesian3 instance from x, y and z coordinates.
    ///
    /// Port of `Cartesian3.fromElements`.
    pub fn from_elements(x: f64, y: f64, z: f64, result: &mut Self) {
        result.x = x;
        result.y = y;
        result.z = z;
    }

    /// Allocating variant of [`Cartesian3::from_elements`].
    pub fn from_elements_new(x: f64, y: f64, z: f64) -> Self {
        Self::new(x, y, z)
    }

    /// Creates a Cartesian3 instance from an existing Cartesian4. This
    /// simply takes the x, y, and z properties of the Cartesian4 and
    /// drops w.
    ///
    /// Port of `Cartesian3.fromCartesian4` (which is aliased to
    /// `Cartesian3.clone` in JS via duck typing; made explicit in Rust).
    pub fn from_cartesian4(cartesian: &Cartesian4, result: &mut Self) {
        result.x = cartesian.x;
        result.y = cartesian.y;
        result.z = cartesian.z;
    }

    /// Allocating variant of [`Cartesian3::from_cartesian4`].
    pub fn from_cartesian4_new(cartesian: &Cartesian4) -> Self {
        Self::new(cartesian.x, cartesian.y, cartesian.z)
    }

    /// Duplicates a Cartesian3 instance into `result`.
    ///
    /// Port of `Cartesian3.clone`. The JS `undefined` input case is
    /// statically impossible in Rust; the prototype `clone` maps to the
    /// derived `Clone` trait.
    pub fn clone_into(cartesian: &Self, result: &mut Self) {
        result.x = cartesian.x;
        result.y = cartesian.y;
        result.z = cartesian.z;
    }

    /// Stores the provided instance into the provided array.
    ///
    /// Port of `Cartesian3.pack`. Returns nothing; the caller already
    /// owns `array` (JS returns the same array reference).
    pub fn pack(value: &Self, array: &mut [f64], starting_index: Option<usize>) {
        let starting_index = starting_index.unwrap_or(0);
        array[starting_index] = value.x;
        array[starting_index + 1] = value.y;
        array[starting_index + 2] = value.z;
    }

    /// Retrieves an instance from a packed array.
    ///
    /// Port of `Cartesian3.unpack`.
    pub fn unpack(array: &[f64], starting_index: Option<usize>, result: &mut Self) {
        let starting_index = starting_index.unwrap_or(0);
        result.x = array[starting_index];
        result.y = array[starting_index + 1];
        result.z = array[starting_index + 2];
    }

    /// Allocating variant of [`Cartesian3::unpack`].
    pub fn unpack_new(array: &[f64], starting_index: Option<usize>) -> Self {
        let mut result = Self::default();
        Self::unpack(array, starting_index, &mut result);
        result
    }

    /// Creates a Cartesian3 from three consecutive elements in an array.
    ///
    /// Port of `Cartesian3.fromArray` (aliased to `Cartesian3.unpack`).
    pub fn from_array(array: &[f64], starting_index: Option<usize>, result: &mut Self) {
        Self::unpack(array, starting_index, result);
    }

    /// Allocating variant of [`Cartesian3::from_array`].
    pub fn from_array_new(array: &[f64], starting_index: Option<usize>) -> Self {
        Self::unpack_new(array, starting_index)
    }

    /// Flattens an array of Cartesian3s into an array of components.
    ///
    /// Port of `Cartesian3.packArray`.
    ///
    /// DEVIATION: JS distinguishes regular arrays (resized) from typed
    /// arrays (must have exactly `array.length * 3` elements, else
    /// `DeveloperError`). A Rust `Vec<f64>` maps to the resizable
    /// regular-array branch; the typed-array exact-length error cannot
    /// be expressed and its spec case is `#[ignore]`d.
    pub fn pack_array(array: &[Self], result: Option<Vec<f64>>) -> Vec<f64> {
        let length = array.len();
        let result_length = length * 3;
        let mut result = match result {
            Some(result) => result,
            None => vec![0.0; result_length],
        };
        result.resize(result_length, 0.0);

        for (i, value) in array.iter().enumerate() {
            Self::pack(value, &mut result, Some(i * 3));
        }

        result
    }

    /// Unpacks an array of cartesian components into an array of
    /// Cartesian3s.
    ///
    /// Port of `Cartesian3.unpackArray`.
    pub fn unpack_array(array: &[f64], result: Option<Vec<Self>>) -> Vec<Self> {
        if cfg!(debug_assertions) {
            check::type_of::number_greater_than_or_equals(
                "array.length",
                array.len() as f64,
                3.0,
            );
            if array.len() % 3 != 0 {
                throw_developer_error("array length must be a multiple of 3.");
            }
        }

        let length = array.len();
        let mut result = match result {
            Some(result) => result,
            None => vec![Self::default(); length / 3],
        };
        result.resize(length / 3, Self::default());

        let mut i = 0;
        while i < length {
            let index = i / 3;
            Self::unpack(array, Some(i), &mut result[index]);
            i += 3;
        }
        result
    }

    /// Computes the value of the maximum component for the supplied
    /// Cartesian.
    ///
    /// Port of `Cartesian3.maximumComponent`.
    pub fn maximum_component(cartesian: &Self) -> f64 {
        cartesian.x.max(cartesian.y).max(cartesian.z)
    }

    /// Computes the value of the minimum component for the supplied
    /// Cartesian.
    ///
    /// Port of `Cartesian3.minimumComponent`.
    pub fn minimum_component(cartesian: &Self) -> f64 {
        cartesian.x.min(cartesian.y).min(cartesian.z)
    }

    /// Compares two Cartesians and computes a Cartesian which contains
    /// the minimum components of the supplied Cartesians.
    ///
    /// Port of `Cartesian3.minimumByComponent`.
    pub fn minimum_by_component(first: &Self, second: &Self, result: &mut Self) {
        result.x = first.x.min(second.x);
        result.y = first.y.min(second.y);
        result.z = first.z.min(second.z);
    }

    /// Allocating variant of [`Cartesian3::minimum_by_component`].
    pub fn minimum_by_component_new(first: &Self, second: &Self) -> Self {
        let mut result = Self::default();
        Self::minimum_by_component(first, second, &mut result);
        result
    }

    /// Compares two Cartesians and computes a Cartesian which contains
    /// the maximum components of the supplied Cartesians.
    ///
    /// Port of `Cartesian3.maximumByComponent`.
    pub fn maximum_by_component(first: &Self, second: &Self, result: &mut Self) {
        result.x = first.x.max(second.x);
        result.y = first.y.max(second.y);
        result.z = first.z.max(second.z);
    }

    /// Allocating variant of [`Cartesian3::maximum_by_component`].
    pub fn maximum_by_component_new(first: &Self, second: &Self) -> Self {
        let mut result = Self::default();
        Self::maximum_by_component(first, second, &mut result);
        result
    }

    /// Constrain a value to lie between two values.
    ///
    /// Port of `Cartesian3.clamp`.
    pub fn clamp(value: &Self, min: &Self, max: &Self, result: &mut Self) {
        result.x = CesiumMath::clamp(value.x, min.x, max.x);
        result.y = CesiumMath::clamp(value.y, min.y, max.y);
        result.z = CesiumMath::clamp(value.z, min.z, max.z);
    }

    /// Allocating variant of [`Cartesian3::clamp`].
    pub fn clamp_new(value: &Self, min: &Self, max: &Self) -> Self {
        let mut result = Self::default();
        Self::clamp(value, min, max, &mut result);
        result
    }

    /// Computes the provided Cartesian's squared magnitude.
    ///
    /// Port of `Cartesian3.magnitudeSquared`.
    pub fn magnitude_squared(cartesian: &Self) -> f64 {
        cartesian.x * cartesian.x + cartesian.y * cartesian.y + cartesian.z * cartesian.z
    }

    /// Computes the Cartesian's magnitude (length).
    ///
    /// Port of `Cartesian3.magnitude`.
    pub fn magnitude(cartesian: &Self) -> f64 {
        Self::magnitude_squared(cartesian).sqrt()
    }

    /// Computes the distance between two points.
    ///
    /// Port of `Cartesian3.distance`.
    pub fn distance(left: &Self, right: &Self) -> f64 {
        let mut distance_scratch = Self::default();
        Self::subtract(left, right, &mut distance_scratch);
        Self::magnitude(&distance_scratch)
    }

    /// Computes the squared distance between two points. Comparing
    /// squared distances using this function is more efficient than
    /// comparing distances using [`Cartesian3::distance`].
    ///
    /// Port of `Cartesian3.distanceSquared`.
    pub fn distance_squared(left: &Self, right: &Self) -> f64 {
        let mut distance_scratch = Self::default();
        Self::subtract(left, right, &mut distance_scratch);
        Self::magnitude_squared(&distance_scratch)
    }

    /// Computes the normalized form of the supplied Cartesian.
    ///
    /// Port of `Cartesian3.normalize`.
    pub fn normalize(cartesian: &Self, result: &mut Self) {
        let magnitude = Self::magnitude(cartesian);

        result.x = cartesian.x / magnitude;
        result.y = cartesian.y / magnitude;
        result.z = cartesian.z / magnitude;

        if cfg!(debug_assertions) {
            if result.x.is_nan() || result.y.is_nan() || result.z.is_nan() {
                throw_developer_error("normalized result is not a number");
            }
        }
    }

    /// Allocating variant of [`Cartesian3::normalize`].
    pub fn normalize_new(cartesian: &Self) -> Self {
        let mut result = Self::default();
        Self::normalize(cartesian, &mut result);
        result
    }

    /// Computes the dot (scalar) product of two Cartesians.
    ///
    /// Port of `Cartesian3.dot`.
    pub fn dot(left: &Self, right: &Self) -> f64 {
        left.x * right.x + left.y * right.y + left.z * right.z
    }

    /// Computes the componentwise product of two Cartesians.
    ///
    /// Port of `Cartesian3.multiplyComponents`.
    pub fn multiply_components(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x * right.x;
        result.y = left.y * right.y;
        result.z = left.z * right.z;
    }

    /// Allocating variant of [`Cartesian3::multiply_components`].
    pub fn multiply_components_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::multiply_components(left, right, &mut result);
        result
    }

    /// Computes the componentwise quotient of two Cartesians.
    ///
    /// Port of `Cartesian3.divideComponents`.
    pub fn divide_components(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x / right.x;
        result.y = left.y / right.y;
        result.z = left.z / right.z;
    }

    /// Allocating variant of [`Cartesian3::divide_components`].
    pub fn divide_components_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::divide_components(left, right, &mut result);
        result
    }

    /// Computes the componentwise sum of two Cartesians.
    ///
    /// Port of `Cartesian3.add`.
    pub fn add(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x + right.x;
        result.y = left.y + right.y;
        result.z = left.z + right.z;
    }

    /// Allocating variant of [`Cartesian3::add`].
    pub fn add_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::add(left, right, &mut result);
        result
    }

    /// Computes the componentwise difference of two Cartesians.
    ///
    /// Port of `Cartesian3.subtract`.
    pub fn subtract(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x - right.x;
        result.y = left.y - right.y;
        result.z = left.z - right.z;
    }

    /// Allocating variant of [`Cartesian3::subtract`].
    pub fn subtract_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::subtract(left, right, &mut result);
        result
    }

    /// Multiplies the provided Cartesian componentwise by the provided
    /// scalar.
    ///
    /// Port of `Cartesian3.multiplyByScalar`.
    pub fn multiply_by_scalar(cartesian: &Self, scalar: f64, result: &mut Self) {
        result.x = cartesian.x * scalar;
        result.y = cartesian.y * scalar;
        result.z = cartesian.z * scalar;
    }

    /// Allocating variant of [`Cartesian3::multiply_by_scalar`].
    pub fn multiply_by_scalar_new(cartesian: &Self, scalar: f64) -> Self {
        let mut result = Self::default();
        Self::multiply_by_scalar(cartesian, scalar, &mut result);
        result
    }

    /// Divides the provided Cartesian componentwise by the provided
    /// scalar.
    ///
    /// Port of `Cartesian3.divideByScalar`.
    pub fn divide_by_scalar(cartesian: &Self, scalar: f64, result: &mut Self) {
        result.x = cartesian.x / scalar;
        result.y = cartesian.y / scalar;
        result.z = cartesian.z / scalar;
    }

    /// Allocating variant of [`Cartesian3::divide_by_scalar`].
    pub fn divide_by_scalar_new(cartesian: &Self, scalar: f64) -> Self {
        let mut result = Self::default();
        Self::divide_by_scalar(cartesian, scalar, &mut result);
        result
    }

    /// Negates the provided Cartesian.
    ///
    /// Port of `Cartesian3.negate`.
    pub fn negate(cartesian: &Self, result: &mut Self) {
        result.x = -cartesian.x;
        result.y = -cartesian.y;
        result.z = -cartesian.z;
    }

    /// Allocating variant of [`Cartesian3::negate`].
    pub fn negate_new(cartesian: &Self) -> Self {
        let mut result = Self::default();
        Self::negate(cartesian, &mut result);
        result
    }

    /// Computes the absolute value of the provided Cartesian.
    ///
    /// Port of `Cartesian3.abs`.
    pub fn abs(cartesian: &Self, result: &mut Self) {
        result.x = cartesian.x.abs();
        result.y = cartesian.y.abs();
        result.z = cartesian.z.abs();
    }

    /// Allocating variant of [`Cartesian3::abs`].
    pub fn abs_new(cartesian: &Self) -> Self {
        let mut result = Self::default();
        Self::abs(cartesian, &mut result);
        result
    }

    /// Computes the linear interpolation or extrapolation at t using the
    /// provided cartesians.
    ///
    /// Port of `Cartesian3.lerp`.
    pub fn lerp(start: &Self, end: &Self, t: f64, result: &mut Self) {
        let mut lerp_scratch = Self::default();
        Self::multiply_by_scalar(end, t, &mut lerp_scratch);
        Self::multiply_by_scalar(start, 1.0 - t, result);
        // `result` aliases itself as input; copy first (values are Copy).
        let current = *result;
        Self::add(&lerp_scratch, &current, result);
    }

    /// Allocating variant of [`Cartesian3::lerp`].
    pub fn lerp_new(start: &Self, end: &Self, t: f64) -> Self {
        let mut result = Self::default();
        Self::lerp(start, end, t, &mut result);
        result
    }

    /// Returns the angle, in radians, between the provided Cartesians.
    ///
    /// Port of `Cartesian3.angleBetween`.
    pub fn angle_between(left: &Self, right: &Self) -> f64 {
        let mut angle_between_scratch = Self::default();
        let mut angle_between_scratch2 = Self::default();
        Self::normalize(left, &mut angle_between_scratch);
        Self::normalize(right, &mut angle_between_scratch2);
        let cosine = Self::dot(&angle_between_scratch, &angle_between_scratch2);
        let mut cross_scratch = Self::default();
        Self::cross(
            &angle_between_scratch,
            &angle_between_scratch2,
            &mut cross_scratch,
        );
        let sine = Self::magnitude(&cross_scratch);
        sine.atan2(cosine)
    }

    /// Returns the axis that is most orthogonal to the provided
    /// Cartesian.
    ///
    /// Port of `Cartesian3.mostOrthogonalAxis`.
    pub fn most_orthogonal_axis(cartesian: &Self, result: &mut Self) {
        let mut most_orthogonal_axis_scratch = Self::default();
        Self::normalize(cartesian, &mut most_orthogonal_axis_scratch);
        let normalized = most_orthogonal_axis_scratch;
        Self::abs(&normalized, &mut most_orthogonal_axis_scratch);
        let f = most_orthogonal_axis_scratch;

        let unit = if f.x <= f.y {
            if f.x <= f.z {
                Self::UNIT_X
            } else {
                Self::UNIT_Z
            }
        } else if f.y <= f.z {
            Self::UNIT_Y
        } else {
            Self::UNIT_Z
        };
        Self::clone_into(&unit, result);
    }

    /// Projects vector a onto vector b.
    ///
    /// Port of `Cartesian3.projectVector`.
    pub fn project_vector(a: &Self, b: &Self, result: &mut Self) {
        let scalar = Self::dot(a, b) / Self::dot(b, b);
        Self::multiply_by_scalar(b, scalar, result);
    }

    /// Allocating variant of [`Cartesian3::project_vector`].
    pub fn project_vector_new(a: &Self, b: &Self) -> Self {
        let mut result = Self::default();
        Self::project_vector(a, b, &mut result);
        result
    }

    /// Compares the provided Cartesians componentwise and returns true
    /// if they are equal, false otherwise.
    ///
    /// Port of `Cartesian3.equals`. `None` mirrors JS `undefined`.
    pub fn equals(left: Option<&Self>, right: Option<&Self>) -> bool {
        match (left, right) {
            (Some(left), Some(right)) => {
                left.x == right.x && left.y == right.y && left.z == right.z
            }
            (None, None) => true,
            _ => false,
        }
    }

    /// Port of `Cartesian3.equalsArray` (`@ignore` in JS).
    pub fn equals_array(cartesian: &Self, array: &[f64], offset: usize) -> bool {
        cartesian.x == array[offset]
            && cartesian.y == array[offset + 1]
            && cartesian.z == array[offset + 2]
    }

    /// Compares the provided Cartesians componentwise and returns true
    /// if they pass an absolute or relative tolerance test, false
    /// otherwise.
    ///
    /// Port of `Cartesian3.equalsEpsilon`. `None` mirrors JS
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
            }
            (None, None) => true,
            _ => false,
        }
    }

    /// Computes the cross (outer) product of two Cartesians.
    ///
    /// Port of `Cartesian3.cross`. Uses locals so `result` may alias
    /// `left`/`right` exactly like the JS version.
    pub fn cross(left: &Self, right: &Self, result: &mut Self) {
        let left_x = left.x;
        let left_y = left.y;
        let left_z = left.z;
        let right_x = right.x;
        let right_y = right.y;
        let right_z = right.z;

        let x = left_y * right_z - left_z * right_y;
        let y = left_z * right_x - left_x * right_z;
        let z = left_x * right_y - left_y * right_x;

        result.x = x;
        result.y = y;
        result.z = z;
    }

    /// Allocating variant of [`Cartesian3::cross`].
    pub fn cross_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::cross(left, right, &mut result);
        result
    }

    /// Computes the midpoint between the right and left Cartesian.
    ///
    /// Port of `Cartesian3.midpoint`.
    pub fn midpoint(left: &Self, right: &Self, result: &mut Self) {
        result.x = (left.x + right.x) * 0.5;
        result.y = (left.y + right.y) * 0.5;
        result.z = (left.z + right.z) * 0.5;
    }

    /// Allocating variant of [`Cartesian3::midpoint`].
    pub fn midpoint_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::midpoint(left, right, &mut result);
        result
    }

    /// Returns a Cartesian3 position from longitude and latitude values
    /// given in degrees.
    ///
    /// Port of `Cartesian3.fromDegrees`.
    ///
    /// DEVIATION: the JS `ellipsoid` parameter (default
    /// `Ellipsoid.default`) is mapped to `radii_squared` — the ellipsoid
    /// `radiiSquared` vector — because `Ellipsoid` is ported in a later
    /// batch. `None` uses the WGS84 default below.
    pub fn from_degrees(
        longitude: f64,
        latitude: f64,
        height: Option<f64>,
        radii_squared: Option<&Self>,
        result: &mut Self,
    ) {
        let longitude = CesiumMath::to_radians(longitude);
        let latitude = CesiumMath::to_radians(latitude);
        Self::from_radians(longitude, latitude, height, radii_squared, result);
    }

    /// Allocating variant of [`Cartesian3::from_degrees`].
    pub fn from_degrees_new(
        longitude: f64,
        latitude: f64,
        height: Option<f64>,
        radii_squared: Option<&Self>,
    ) -> Self {
        let mut result = Self::default();
        Self::from_degrees(longitude, latitude, height, radii_squared, &mut result);
        result
    }

    /// Returns a Cartesian3 position from longitude and latitude values
    /// given in radians.
    ///
    /// Port of `Cartesian3.fromRadians`. See the DEVIATION note on
    /// [`Cartesian3::from_degrees`] about `radii_squared`.
    pub fn from_radians(
        longitude: f64,
        latitude: f64,
        height: Option<f64>,
        radii_squared: Option<&Self>,
        result: &mut Self,
    ) {
        let height = height.unwrap_or(0.0);

        let default_radii_squared = ellipsoid_radii_squared();
        let radii_squared = radii_squared.unwrap_or(&default_radii_squared);

        let cos_latitude = latitude.cos();
        let mut scratch_n = Self::new(
            cos_latitude * longitude.cos(),
            cos_latitude * longitude.sin(),
            latitude.sin(),
        );
        let scratch_n_in = scratch_n;
        Self::normalize(&scratch_n_in, &mut scratch_n);

        let mut scratch_k = Self::default();
        Self::multiply_components(radii_squared, &scratch_n, &mut scratch_k);
        let gamma = Self::dot(&scratch_n, &scratch_k).sqrt();
        let scratch_k_in = scratch_k;
        Self::divide_by_scalar(&scratch_k_in, gamma, &mut scratch_k);
        let scratch_n_in = scratch_n;
        Self::multiply_by_scalar(&scratch_n_in, height, &mut scratch_n);

        Self::add(&scratch_k, &scratch_n, result);
    }

    /// Allocating variant of [`Cartesian3::from_radians`].
    pub fn from_radians_new(
        longitude: f64,
        latitude: f64,
        height: Option<f64>,
        radii_squared: Option<&Self>,
    ) -> Self {
        let mut result = Self::default();
        Self::from_radians(longitude, latitude, height, radii_squared, &mut result);
        result
    }

    /// Returns an array of Cartesian3 positions given an array of
    /// longitude and latitude values given in degrees.
    ///
    /// Port of `Cartesian3.fromDegreesArray`.
    pub fn from_degrees_array(
        coordinates: &[f64],
        radii_squared: Option<&Self>,
        result: Option<Vec<Self>>,
    ) -> Vec<Self> {
        if cfg!(debug_assertions) {
            if coordinates.len() < 2 || coordinates.len() % 2 != 0 {
                throw_developer_error(
                    "the number of coordinates must be a multiple of 2 and at least 2",
                );
            }
        }

        let length = coordinates.len();
        let mut result = match result {
            Some(result) => result,
            None => vec![Self::default(); length / 2],
        };
        result.resize(length / 2, Self::default());

        let mut i = 0;
        while i < length {
            let longitude = coordinates[i];
            let latitude = coordinates[i + 1];
            let index = i / 2;
            Self::from_degrees(
                longitude,
                latitude,
                Some(0.0),
                radii_squared,
                &mut result[index],
            );
            i += 2;
        }

        result
    }

    /// Returns an array of Cartesian3 positions given an array of
    /// longitude and latitude values given in radians.
    ///
    /// Port of `Cartesian3.fromRadiansArray`.
    pub fn from_radians_array(
        coordinates: &[f64],
        radii_squared: Option<&Self>,
        result: Option<Vec<Self>>,
    ) -> Vec<Self> {
        if cfg!(debug_assertions) {
            if coordinates.len() < 2 || coordinates.len() % 2 != 0 {
                throw_developer_error(
                    "the number of coordinates must be a multiple of 2 and at least 2",
                );
            }
        }

        let length = coordinates.len();
        let mut result = match result {
            Some(result) => result,
            None => vec![Self::default(); length / 2],
        };
        result.resize(length / 2, Self::default());

        let mut i = 0;
        while i < length {
            let longitude = coordinates[i];
            let latitude = coordinates[i + 1];
            let index = i / 2;
            Self::from_radians(
                longitude,
                latitude,
                Some(0.0),
                radii_squared,
                &mut result[index],
            );
            i += 2;
        }

        result
    }

    /// Returns an array of Cartesian3 positions given an array of
    /// longitude, latitude and height values where longitude and
    /// latitude are given in degrees.
    ///
    /// Port of `Cartesian3.fromDegreesArrayHeights`.
    pub fn from_degrees_array_heights(
        coordinates: &[f64],
        radii_squared: Option<&Self>,
        result: Option<Vec<Self>>,
    ) -> Vec<Self> {
        if cfg!(debug_assertions) {
            if coordinates.len() < 3 || coordinates.len() % 3 != 0 {
                throw_developer_error(
                    "the number of coordinates must be a multiple of 3 and at least 3",
                );
            }
        }

        let length = coordinates.len();
        let mut result = match result {
            Some(result) => result,
            None => vec![Self::default(); length / 3],
        };
        result.resize(length / 3, Self::default());

        let mut i = 0;
        while i < length {
            let longitude = coordinates[i];
            let latitude = coordinates[i + 1];
            let height = coordinates[i + 2];
            let index = i / 3;
            Self::from_degrees(
                longitude,
                latitude,
                Some(height),
                radii_squared,
                &mut result[index],
            );
            i += 3;
        }

        result
    }

    /// Returns an array of Cartesian3 positions given an array of
    /// longitude, latitude and height values where longitude and
    /// latitude are given in radians.
    ///
    /// Port of `Cartesian3.fromRadiansArrayHeights`.
    pub fn from_radians_array_heights(
        coordinates: &[f64],
        radii_squared: Option<&Self>,
        result: Option<Vec<Self>>,
    ) -> Vec<Self> {
        if cfg!(debug_assertions) {
            if coordinates.len() < 3 || coordinates.len() % 3 != 0 {
                throw_developer_error(
                    "the number of coordinates must be a multiple of 3 and at least 3",
                );
            }
        }

        let length = coordinates.len();
        let mut result = match result {
            Some(result) => result,
            None => vec![Self::default(); length / 3],
        };
        result.resize(length / 3, Self::default());

        let mut i = 0;
        while i < length {
            let longitude = coordinates[i];
            let latitude = coordinates[i + 1];
            let height = coordinates[i + 2];
            let index = i / 3;
            Self::from_radians(
                longitude,
                latitude,
                Some(height),
                radii_squared,
                &mut result[index],
            );
            i += 3;
        }

        result
    }

    /// Compares this Cartesian against the provided Cartesian
    /// componentwise and returns true if they are equal.
    ///
    /// Port of `Cartesian3.prototype.equals`.
    pub fn equals_method(&self, right: &Self) -> bool {
        Self::equals(Some(self), Some(right))
    }

    /// Compares this Cartesian against the provided Cartesian
    /// componentwise and returns true if they pass an absolute or
    /// relative tolerance test.
    ///
    /// Port of `Cartesian3.prototype.equalsEpsilon`.
    pub fn equals_epsilon_method(
        &self,
        right: &Self,
        relative_epsilon: Option<f64>,
        absolute_epsilon: Option<f64>,
    ) -> bool {
        Self::equals_epsilon(Some(self), Some(right), relative_epsilon, absolute_epsilon)
    }

}

impl Default for Cartesian3 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl fmt::Display for Cartesian3 {
    /// Port of `Cartesian3.prototype.toString` — format `(x, y, z)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

// To prevent a circular dependency, this value is overridden by Ellipsoid
// when `Ellipsoid.default` is set (JS: `Cartesian3._ellipsoidRadiiSquared`).
static ELLIPSOID_RADII_SQUARED: Mutex<Cartesian3> = Mutex::new(Cartesian3::new(
    6378137.0 * 6378137.0,
    6378137.0 * 6378137.0,
    6356752.3142451793 * 6356752.3142451793,
));

/// Returns the current default ellipsoid `radiiSquared` used by
/// [`Cartesian3::from_radians`] when no ellipsoid is supplied.
///
/// DEVIATION: public because the `Ellipsoid` port (later batch) and the
/// spec mirrors need to observe the JS module-level mutable default.
pub fn ellipsoid_radii_squared() -> Cartesian3 {
    *ELLIPSOID_RADII_SQUARED.lock().unwrap()
}

/// Overrides the default ellipsoid `radiiSquared`.
///
/// Called by `Ellipsoid` when `Ellipsoid.default` is set, mirroring the
/// JS assignment `Cartesian3._ellipsoidRadiiSquared = ...`.
///
/// DEVIATION: public so the `Ellipsoid` port and spec mirrors can emulate
/// `Ellipsoid.default = ...` (which reassigns this value in JS).
pub fn set_ellipsoid_radii_squared(radii_squared: Cartesian3) {
    *ELLIPSOID_RADII_SQUARED.lock().unwrap() = radii_squared;
}
