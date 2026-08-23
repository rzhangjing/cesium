//! Ported from `packages/engine/Source/Scene/QuadtreeOccluders.js`.
//!
//! Occluders used for culling quadtree tiles that are not visible.

use cesium_core::ellipsoid::Ellipsoid;

/// Occluders used for culling quadtree tiles that are not visible.
///
/// Uses the ellipsoid horizon culling algorithm to determine if a tile
/// is potentially visible from the camera position.
pub struct QuadtreeOccluders {
    /// The ellipsoid used for horizon culling.
    ellipsoid: Ellipsoid,
}

impl QuadtreeOccluders {
    /// Creates a new QuadtreeOccluders.
    pub fn new(ellipsoid: Ellipsoid) -> Self {
        Self { ellipsoid }
    }

    /// Returns the ellipsoid used for horizon culling.
    pub fn ellipsoid(&self) -> &Ellipsoid { &self.ellipsoid }
}

impl Default for QuadtreeOccluders {
    fn default() -> Self { Self::new(Ellipsoid::WGS84) }
}
