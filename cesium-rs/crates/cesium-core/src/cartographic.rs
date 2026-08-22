//! Ported from packages/engine/Source/Core/Cartographic.js
//!
//! A position defined by longitude, latitude, and height.

use std::fmt;
use std::sync::Mutex;

use crate::cartesian3::Cartesian3;
use crate::math::CesiumMath;
use crate::scale_to_geodetic_surface::scale_to_geodetic_surface;

/// Ellipsoid parameter bundle consumed by [`Cartographic::from_cartesian`].
///
/// DEVIATION (deferred): the JS signature takes an `Ellipsoid` instance
/// and reads `oneOverRadii`, `oneOverRadiiSquared` and
/// `_centerToleranceSquared` from it. The `Ellipsoid` port lands in a
/// later batch (W2 ellipsoid/projection group); this struct mirrors
/// exactly the three fields consumed so callers (and the `Ellipsoid`
/// port later) can supply them. Registered in `docs/deferred.md`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EllipsoidParams {
    pub one_over_radii: Cartesian3,
    pub one_over_radii_squared: Cartesian3,
    pub center_tolerance_squared: f64,
}

/// A position defined by longitude, latitude, and height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cartographic {
    /// The longitude, in radians.
    pub longitude: f64,
    /// The latitude, in radians.
    pub latitude: f64,
    /// The height, in meters, above the ellipsoid.
    pub height: f64,
}

impl Cartographic {
    /// An immutable Cartographic instance initialized to (0.0, 0.0, 0.0).
    ///
    /// Port of `Cartographic.ZERO`.
    pub const ZERO: Cartographic = Cartographic::new(0.0, 0.0, 0.0);

    /// Port of the `Cartographic` constructor. `None` arguments are
    /// statically impossible in Rust; the JS `?? 0.0` defaults map to
    /// this constructor's plain parameters.
    pub const fn new(longitude: f64, latitude: f64, height: f64) -> Self {
        Self {
            longitude,
            latitude,
            height,
        }
    }

    /// Creates a new Cartographic instance from longitude and latitude
    /// specified in radians.
    ///
    /// Port of `Cartographic.fromRadians`. `height` mirrors the JS
    /// optional argument (`?? 0.0`). The JS `Check.typeOf.number`
    /// debug checks are statically impossible in Rust.
    pub fn from_radians(longitude: f64, latitude: f64, height: Option<f64>, result: &mut Self) {
        let height = height.unwrap_or(0.0);

        result.longitude = longitude;
        result.latitude = latitude;
        result.height = height;
    }

    /// Allocating variant of [`Cartographic::from_radians`].
    pub fn from_radians_new(longitude: f64, latitude: f64, height: Option<f64>) -> Self {
        let mut result = Self::default();
        Self::from_radians(longitude, latitude, height, &mut result);
        result
    }

    /// Creates a new Cartographic instance from longitude and latitude
    /// specified in degrees. The values in the resulting object will
    /// be in radians.
    ///
    /// Port of `Cartographic.fromDegrees`.
    pub fn from_degrees(longitude: f64, latitude: f64, height: Option<f64>, result: &mut Self) {
        let longitude = CesiumMath::to_radians(longitude);
        let latitude = CesiumMath::to_radians(latitude);

        Self::from_radians(longitude, latitude, height, result);
    }

    /// Allocating variant of [`Cartographic::from_degrees`].
    pub fn from_degrees_new(longitude: f64, latitude: f64, height: Option<f64>) -> Self {
        let mut result = Self::default();
        Self::from_degrees(longitude, latitude, height, &mut result);
        result
    }

    /// Creates a new Cartographic instance from a Cartesian position.
    /// The values in the resulting object will be in radians.
    ///
    /// Port of `Cartographic.fromCartesian`. Returns `false` where the
    /// JS returns `undefined` (the cartesian is at the center of the
    /// ellipsoid). `None` selects the default ellipsoid parameters
    /// (kept in sync with `Ellipsoid.default`, see
    /// [`set_default_ellipsoid`]). The JS "`cartesian is required.`"
    /// check is thrown from `scaleToGeodeticSurface` and statically
    /// impossible in Rust.
    pub fn from_cartesian(
        cartesian: &Cartesian3,
        ellipsoid: Option<&EllipsoidParams>,
        result: &mut Self,
    ) -> bool {
        let default_one_over_radii;
        let default_one_over_radii_squared;
        let default_center_tolerance_squared;
        let (one_over_radii, one_over_radii_squared, center_tolerance_squared) =
            match ellipsoid {
                Some(ellipsoid) => (
                    &ellipsoid.one_over_radii,
                    &ellipsoid.one_over_radii_squared,
                    ellipsoid.center_tolerance_squared,
                ),
                None => {
                    default_one_over_radii = ellipsoid_one_over_radii();
                    default_one_over_radii_squared = ellipsoid_one_over_radii_squared();
                    default_center_tolerance_squared = ellipsoid_center_tolerance_squared();
                    (
                        &default_one_over_radii,
                        &default_one_over_radii_squared,
                        default_center_tolerance_squared,
                    )
                }
            };

        let mut p = Cartesian3::default();
        if !scale_to_geodetic_surface(
            cartesian,
            one_over_radii,
            one_over_radii_squared,
            center_tolerance_squared,
            &mut p,
        ) {
            return false;
        }

        let mut n = Cartesian3::default();
        Cartesian3::multiply_components(&p, one_over_radii_squared, &mut n);
        let n_in = n;
        Cartesian3::normalize(&n_in, &mut n);

        let mut h = Cartesian3::default();
        Cartesian3::subtract(cartesian, &p, &mut h);

        let longitude = n.y.atan2(n.x);
        let latitude = n.z.asin();
        let height = CesiumMath::sign(Cartesian3::dot(&h, cartesian)) * Cartesian3::magnitude(&h);

        result.longitude = longitude;
        result.latitude = latitude;
        result.height = height;
        true
    }

    /// Allocating variant of [`Cartographic::from_cartesian`]; `None`
    /// mirrors the JS `undefined` return.
    pub fn from_cartesian_new(
        cartesian: &Cartesian3,
        ellipsoid: Option<&EllipsoidParams>,
    ) -> Option<Self> {
        let mut result = Self::default();
        if Self::from_cartesian(cartesian, ellipsoid, &mut result) {
            Some(result)
        } else {
            None
        }
    }

    /// Creates a new Cartesian3 instance from a Cartographic input.
    /// The values in the inputted object should be in radians.
    ///
    /// Port of `Cartographic.toCartesian`. The JS `ellipsoid` argument
    /// (defaulting to `Ellipsoid.default`) is threaded through
    /// [`Cartesian3::from_radians`] via the module-level default
    /// `radiiSquared`; explicit ellipsoids map to their
    /// `radiiSquared` there. The JS `Check.defined("cartographic")`
    /// debug check is statically impossible in Rust.
    pub fn to_cartesian(cartographic: &Self, result: &mut Cartesian3) {
        Cartesian3::from_radians(
            cartographic.longitude,
            cartographic.latitude,
            Some(cartographic.height),
            None,
            result,
        );
    }

    /// Allocating variant of [`Cartographic::to_cartesian`].
    pub fn to_cartesian_new(cartographic: &Self) -> Cartesian3 {
        let mut result = Cartesian3::default();
        Self::to_cartesian(cartographic, &mut result);
        result
    }

    /// Duplicates a Cartographic instance into `result`.
    ///
    /// Port of `Cartographic.clone`. The JS `undefined` input case
    /// (returns `undefined`) is statically impossible in Rust; the
    /// prototype `clone` maps to the derived `Clone` trait.
    pub fn clone_into(cartographic: &Self, result: &mut Self) {
        result.longitude = cartographic.longitude;
        result.latitude = cartographic.latitude;
        result.height = cartographic.height;
    }

    /// Compares the provided cartographics componentwise and returns
    /// true if they are equal, false otherwise.
    ///
    /// Port of `Cartographic.equals`. `None` mirrors JS `undefined`.
    pub fn equals(left: Option<&Self>, right: Option<&Self>) -> bool {
        match (left, right) {
            (Some(left), Some(right)) => {
                left.longitude == right.longitude
                    && left.latitude == right.latitude
                    && left.height == right.height
            }
            (None, None) => true,
            _ => false,
        }
    }

    /// Compares the provided cartographics componentwise and returns
    /// true if they are within the provided epsilon, false otherwise.
    ///
    /// Port of `Cartographic.equalsEpsilon` (single absolute epsilon,
    /// `?? 0` — unlike the Cartesian family's relative/absolute pair).
    /// `None` mirrors JS `undefined`.
    pub fn equals_epsilon(left: Option<&Self>, right: Option<&Self>, epsilon: Option<f64>) -> bool {
        let epsilon = epsilon.unwrap_or(0.0);

        match (left, right) {
            (Some(left), Some(right)) => {
                (left.longitude - right.longitude).abs() <= epsilon
                    && (left.latitude - right.latitude).abs() <= epsilon
                    && (left.height - right.height).abs() <= epsilon
            }
            (None, None) => true,
            _ => false,
        }
    }

    /// Compares this cartographic against the provided cartographic
    /// componentwise and returns true if they are equal.
    ///
    /// Port of `Cartographic.prototype.equals`.
    pub fn equals_method(&self, right: &Self) -> bool {
        Self::equals(Some(self), Some(right))
    }

    /// Compares this cartographic against the provided cartographic
    /// componentwise and returns true if they are within the provided
    /// epsilon.
    ///
    /// Port of `Cartographic.prototype.equalsEpsilon`.
    pub fn equals_epsilon_method(&self, right: &Self, epsilon: Option<f64>) -> bool {
        Self::equals_epsilon(Some(self), Some(right), epsilon)
    }
}

impl Default for Cartographic {
    fn default() -> Self {
        Self::ZERO
    }
}

impl fmt::Display for Cartographic {
    /// Port of `Cartographic.prototype.toString` — format
    /// `(longitude, latitude, height)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.longitude, self.latitude, self.height)
    }
}

// To avoid circular dependencies, these are set by Ellipsoid when
// `Ellipsoid.default` is set (JS: `Cartographic._ellipsoidOneOverRadii`
// and friends).
static ELLIPSOID_ONE_OVER_RADII: Mutex<Cartesian3> = Mutex::new(Cartesian3::new(
    1.0 / 6378137.0,
    1.0 / 6378137.0,
    1.0 / 6356752.3142451793,
));

static ELLIPSOID_ONE_OVER_RADII_SQUARED: Mutex<Cartesian3> = Mutex::new(Cartesian3::new(
    1.0 / (6378137.0 * 6378137.0),
    1.0 / (6378137.0 * 6378137.0),
    1.0 / (6356752.3142451793 * 6356752.3142451793),
));

static ELLIPSOID_CENTER_TOLERANCE_SQUARED: Mutex<f64> = Mutex::new(CesiumMath::EPSILON1);

/// Returns the current default ellipsoid `oneOverRadii` used by
/// [`Cartographic::from_cartesian`] when no ellipsoid is supplied.
pub fn ellipsoid_one_over_radii() -> Cartesian3 {
    *ELLIPSOID_ONE_OVER_RADII.lock().unwrap()
}

/// Returns the current default ellipsoid `oneOverRadiiSquared`.
pub fn ellipsoid_one_over_radii_squared() -> Cartesian3 {
    *ELLIPSOID_ONE_OVER_RADII_SQUARED.lock().unwrap()
}

/// Returns the current default ellipsoid `_centerToleranceSquared`.
pub fn ellipsoid_center_tolerance_squared() -> f64 {
    *ELLIPSOID_CENTER_TOLERANCE_SQUARED.lock().unwrap()
}

/// Overrides the default ellipsoid parameters.
///
/// Called by `Ellipsoid` when `Ellipsoid.default` is set, mirroring the
/// JS assignments to `Cartographic._ellipsoidOneOverRadii` and friends.
///
/// DEVIATION: public so the `Ellipsoid` port and spec mirrors can
/// emulate `Ellipsoid.default = ...` (which reassigns these values in
/// JS).
pub fn set_default_ellipsoid(params: EllipsoidParams) {
    *ELLIPSOID_ONE_OVER_RADII.lock().unwrap() = params.one_over_radii;
    *ELLIPSOID_ONE_OVER_RADII_SQUARED.lock().unwrap() = params.one_over_radii_squared;
    *ELLIPSOID_CENTER_TOLERANCE_SQUARED.lock().unwrap() = params.center_tolerance_squared;
}
