//! Ported from `packages/engine/Source/DataSources/GroundGeometryUpdater.js`.

use crate::entity::Entity;
use crate::geometry_updater::GeometryUpdater;

/// A geometry updater for geometry that conforms to the terrain surface.
///
/// Ground geometry updaters handle the process of draping geometry
/// onto the terrain, including terrain-aware occlusion.
pub struct GroundGeometryUpdater {
    entity_id: String,
    fill_enabled: bool,
    outline_enabled: bool,
    clamp_to_ground: bool,
}

impl GroundGeometryUpdater {
    /// Creates a new ground geometry updater.
    pub fn new(entity: &Entity, clamp_to_ground: bool) -> Self {
        Self {
            entity_id: entity.id.clone(),
            fill_enabled: true,
            outline_enabled: false,
            clamp_to_ground,
        }
    }

    /// Gets whether this geometry is clamped to the ground.
    pub fn clamp_to_ground(&self) -> bool { self.clamp_to_ground }
}

impl GeometryUpdater for GroundGeometryUpdater {
    fn entity_id(&self) -> &str { &self.entity_id }
    fn fill_enabled(&self) -> bool { self.fill_enabled }
    fn outline_enabled(&self) -> bool { self.outline_enabled }
    fn is_on_surface(&self) -> bool { self.clamp_to_ground }
    fn is_closed(&self) -> bool { false }
}
