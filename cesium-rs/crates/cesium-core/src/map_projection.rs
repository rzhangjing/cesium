//! Ported from `packages/engine/Source/Core/MapProjection.js`.

use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;

/// Defines how geodetic ellipsoid coordinates project to a flat map.
pub trait MapProjection {
    /// Gets the ellipsoid.
    fn ellipsoid(&self) -> &Ellipsoid;

    /// Projects cartographic coordinates to projection-specific map coordinates.
    fn project(&self, cartographic: &Cartographic) -> Cartesian3;

    /// Unprojects map coordinates to cartographic coordinates.
    fn unproject(&self, cartesian: &Cartesian3) -> Cartographic;
}
