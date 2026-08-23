//! Ported from `packages/engine/Source/Scene/ClippingPlane.js`.
//!
//! A single clipping plane.

use cesium_core::cartesian3::Cartesian3;

/// A plane used for clipping geometry.
///
/// Defined by a normal direction and distance from the origin.
/// Mirrors CesiumJS `ClippingPlane` (100 lines).
pub struct ClippingPlane {
    /// The normal direction of the plane.
    pub normal: Cartesian3,
    /// The distance from the origin along the normal.
    pub distance: f64,
}

impl ClippingPlane {
    /// Creates a new ClippingPlane.
    pub fn new(normal: Cartesian3, distance: f64) -> Self {
        Self { normal, distance }
    }
}

impl Default for ClippingPlane {
    fn default() -> Self {
        Self {
            normal: Cartesian3::new(0.0, 0.0, 1.0),
            distance: 0.0,
        }
    }
}
