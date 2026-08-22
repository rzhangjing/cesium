//! Ported from `packages/engine/Source/Core/TerrainPicker.js`.
//!
//! Terrain picking via ray intersection. Full implementation deferred.

use crate::cartesian3::Cartesian3;
use crate::ray::Ray;

/// Provides ray intersection testing against terrain meshes.
pub struct TerrainPicker {
    /// Whether the internal data structures need rebuilding.
    pub needs_rebuild: bool,
}

impl TerrainPicker {
    /// Creates a new TerrainPicker.
    pub fn new() -> Self {
        Self {
            needs_rebuild: true,
        }
    }

    /// Tests a ray against the terrain mesh.
    /// Returns the intersection point, or None.
    pub fn ray_intersect(&self, _ray: &Ray) -> Option<Cartesian3> {
        // TODO: full implementation
        None
    }
}

impl Default for TerrainPicker {
    fn default() -> Self {
        Self::new()
    }
}
