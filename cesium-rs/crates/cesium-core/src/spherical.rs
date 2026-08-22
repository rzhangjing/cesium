//! Ported from packages/engine/Source/Core/Spherical.js
//!
//! A set of curvilinear 3-dimensional coordinates.

use std::fmt;

use crate::cartesian3::Cartesian3;

/// A set of curvilinear 3-dimensional coordinates.
///
/// Port of `Spherical`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spherical {
    /// The clock component.
    pub clock: f64,
    /// The cone component.
    pub cone: f64,
    /// The magnitude component.
    pub magnitude: f64,
}

impl Spherical {
    /// Creates a new `Spherical`.
    ///
    /// Port of the `Spherical(clock, cone, magnitude)` constructor. JS
    /// defaults: `clock = 0.0`, `cone = 0.0`, `magnitude = 1.0`.
    pub const fn new(clock: f64, cone: f64, magnitude: f64) -> Self {
        Self {
            clock,
            cone,
            magnitude,
        }
    }

    /// Converts the provided Cartesian3 into Spherical coordinates.
    ///
    /// Port of `Spherical.fromCartesian3`.
    pub fn from_cartesian3(cartesian3: &Cartesian3, result: &mut Self) {
        let x = cartesian3.x;
        let y = cartesian3.y;
        let z = cartesian3.z;
        let radial_squared = x * x + y * y;

        result.clock = y.atan2(x);
        result.cone = radial_squared.sqrt().atan2(z);
        result.magnitude = (radial_squared + z * z).sqrt();
    }

    /// Allocating variant of [`Spherical::from_cartesian3`].
    pub fn from_cartesian3_new(cartesian3: &Cartesian3) -> Self {
        let mut result = Self::default();
        Self::from_cartesian3(cartesian3, &mut result);
        result
    }

    /// Creates a duplicate of a `Spherical` into `result`.
    ///
    /// Port of `Spherical.clone`. The JS `undefined` input case is
    /// statically impossible in Rust (see DEVIATION notes in
    /// PORTING_CONVENTIONS.md); the prototype `clone` maps to the
    /// derived `Clone` trait.
    pub fn clone_into(spherical: &Self, result: &mut Self) {
        result.clock = spherical.clock;
        result.cone = spherical.cone;
        result.magnitude = spherical.magnitude;
    }

    /// Computes the normalized version of the provided spherical.
    ///
    /// Port of `Spherical.normalize`.
    pub fn normalize(spherical: &Self, result: &mut Self) {
        result.clock = spherical.clock;
        result.cone = spherical.cone;
        result.magnitude = 1.0;
    }

    /// Allocating variant of [`Spherical::normalize`].
    pub fn normalize_new(spherical: &Self) -> Self {
        let mut result = Self::default();
        Self::normalize(spherical, &mut result);
        result
    }

    /// Returns true if the first spherical is equal to the second
    /// spherical, false otherwise.
    ///
    /// Port of `Spherical.equals`. `None` mirrors JS `undefined`.
    pub fn equals(left: Option<&Self>, right: Option<&Self>) -> bool {
        match (left, right) {
            (Some(left), Some(right)) => {
                left.clock == right.clock
                    && left.cone == right.cone
                    && left.magnitude == right.magnitude
            }
            (None, None) => true,
            _ => false,
        }
    }

    /// Returns true if the first spherical is within the provided epsilon
    /// of the second spherical, false otherwise.
    ///
    /// Port of `Spherical.equalsEpsilon`. `epsilon` defaults to `0.0`
    /// when `None` (JS `epsilon ?? 0.0`).
    pub fn equals_epsilon(
        left: Option<&Self>,
        right: Option<&Self>,
        epsilon: Option<f64>,
    ) -> bool {
        let epsilon = epsilon.unwrap_or(0.0);
        match (left, right) {
            (Some(left), Some(right)) => {
                (left.clock - right.clock).abs() <= epsilon
                    && (left.cone - right.cone).abs() <= epsilon
                    && (left.magnitude - right.magnitude).abs() <= epsilon
            }
            (None, None) => true,
            _ => false,
        }
    }

    /// Returns true if this spherical is equal to the provided spherical.
    ///
    /// Port of `Spherical.prototype.equals`.
    pub fn equals_method(&self, other: &Self) -> bool {
        Self::equals(Some(self), Some(other))
    }

    /// Returns true if this spherical is within the provided epsilon of
    /// the provided spherical.
    ///
    /// Port of `Spherical.prototype.equalsEpsilon`.
    pub fn equals_epsilon_method(&self, other: &Self, epsilon: f64) -> bool {
        Self::equals_epsilon(Some(self), Some(other), Some(epsilon))
    }
}

impl Default for Spherical {
    fn default() -> Self {
        Self::new(0.0, 0.0, 1.0)
    }
}

impl fmt::Display for Spherical {
    /// Port of `Spherical.prototype.toString` — format
    /// `(clock, cone, magnitude)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.clock, self.cone, self.magnitude)
    }
}
