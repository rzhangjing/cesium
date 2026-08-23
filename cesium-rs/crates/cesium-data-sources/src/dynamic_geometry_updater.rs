//! Ported from `packages/engine/Source/DataSources/DynamicGeometryUpdater.js`.

use crate::entity::Entity;
use crate::geometry_updater::GeometryUpdater;

/// A geometry updater for dynamic (time-varying) geometry.
///
/// Dynamic geometry updaters re-create geometry each frame because
/// the entity's geometry properties may change over time.
pub struct DynamicGeometryUpdater {
    entity_id: String,
    fill_enabled: bool,
    outline_enabled: bool,
}

impl DynamicGeometryUpdater {
    /// Creates a new dynamic geometry updater.
    pub fn new(entity: &Entity) -> Self {
        Self {
            entity_id: entity.id.clone(),
            fill_enabled: true,
            outline_enabled: false,
        }
    }

    /// Updates the dynamic geometry for the given time.
    pub fn update(&mut self, _time: f64) {
        // DEVIATION: Requires re-creation of geometry based on current time
    }
}

impl GeometryUpdater for DynamicGeometryUpdater {
    fn entity_id(&self) -> &str { &self.entity_id }
    fn fill_enabled(&self) -> bool { self.fill_enabled }
    fn outline_enabled(&self) -> bool { self.outline_enabled }
    fn is_on_surface(&self) -> bool { false }
    fn is_closed(&self) -> bool { false }
}
