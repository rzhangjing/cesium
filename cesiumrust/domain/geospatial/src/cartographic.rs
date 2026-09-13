//! Cartographic - a position defined by longitude, latitude, and height.
//! Maps to CesiumJS `Core/Cartographic.js`

use crate::ellipsoid::Ellipsoid;
use crate::math_utils;
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// A position defined by longitude, latitude, and height above the ellipsoid.
/// Longitude and latitude are in radians. Height is in meters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Cartographic {
    /// Longitude in radians.
    pub longitude: f64,
    /// Latitude in radians.
    pub latitude: f64,
    /// Height in meters above the ellipsoid.
    pub height: f64,
}

impl Cartographic {
    /// Creates a new Cartographic from radians.
    /// Maps to Cartographic.fromRadians
    #[inline]
    pub fn from_radians(longitude: f64, latitude: f64, height: f64) -> Self {
        Self {
            longitude,
            latitude,
            height,
        }
    }

    /// Creates a new Cartographic from degrees.
    /// Maps to Cartographic.fromDegrees
    #[inline]
    pub fn from_degrees(longitude: f64, latitude: f64, height: f64) -> Self {
        Self {
            longitude: math_utils::to_radians(longitude),
            latitude: math_utils::to_radians(latitude),
            height,
        }
    }

    /// Creates a Cartographic at the origin (0, 0, 0).
    pub const ZERO: Self = Self {
        longitude: 0.0,
        latitude: 0.0,
        height: 0.0,
    };

    /// Determines if this Cartographic is equal to another within an epsilon.
    pub fn equals_epsilon(&self, other: &Self, epsilon: f64) -> bool {
        (self.longitude - other.longitude).abs() <= epsilon
            && (self.latitude - other.latitude).abs() <= epsilon
            && (self.height - other.height).abs() <= epsilon
    }

    /// Creates a new Cartesian3 instance from a Cartographic input.
    /// Maps to `Cartographic.toCartesian`. The input values are in radians.
    /// The ellipsoid is passed explicitly (Rust has no `Ellipsoid.default` global).
    pub fn to_cartesian(cartographic: &Cartographic, ellipsoid: &Ellipsoid) -> DVec3 {
        ellipsoid.cartographic_to_cartesian(cartographic)
    }

    /// Creates a new Cartographic instance from a Cartesian position.
    /// Maps to `Cartographic.fromCartesian`. The resulting values are in radians.
    /// The ellipsoid is passed explicitly (Rust has no `Ellipsoid.default` global).
    /// Returns None if the cartesian is at the center of the ellipsoid.
    pub fn from_cartesian(cartesian: DVec3, ellipsoid: &Ellipsoid) -> Option<Cartographic> {
        ellipsoid.cartesian_to_cartographic(cartesian)
    }
}

impl std::fmt::Display for Cartographic {
    /// Maps to `Cartographic.toString` → `(longitude, latitude, height)`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.longitude, self.latitude, self.height)
    }
}

impl Default for Cartographic {
    fn default() -> Self {
        Self::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_from_degrees() {
        let c = Cartographic::from_degrees(180.0, 90.0, 1000.0);
        assert!((c.longitude - PI).abs() < 1e-15);
        assert!((c.latitude - PI / 2.0).abs() < 1e-15);
        assert_eq!(c.height, 1000.0);
    }

    #[test]
    fn test_from_radians() {
        let c = Cartographic::from_radians(1.0, 0.5, 200.0);
        assert_eq!(c.longitude, 1.0);
        assert_eq!(c.latitude, 0.5);
        assert_eq!(c.height, 200.0);
    }

    #[test]
    fn test_equals_epsilon() {
        let a = Cartographic::from_radians(1.0, 0.5, 100.0);
        let b = Cartographic::from_radians(1.0 + 1e-12, 0.5, 100.0);
        assert!(a.equals_epsilon(&b, 1e-10));
        assert!(!a.equals_epsilon(&b, 1e-14));
    }
}
