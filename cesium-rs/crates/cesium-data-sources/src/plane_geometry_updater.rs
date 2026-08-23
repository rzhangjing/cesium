//! Ported from `packages/engine/Source/DataSources/PlaneGeometryUpdater.js`.

use crate::entity::Entity;
use crate::geometry_updater::GeometryUpdater;

/// A geometry updater that creates plane geometry instances from entity data.
pub struct PlaneGeometryUpdater {
    entity_id: String,
    fill_enabled: bool,
    outline_enabled: bool,
}

impl PlaneGeometryUpdater {
    /// Creates a new plane geometry updater.
    pub fn new(entity: &Entity) -> Self {
        Self {
            entity_id: entity.id.clone(),
            fill_enabled: true,
            outline_enabled: false,
        }
    }

    /// Creates the geometry instance for fill.
    pub fn create_fill_geometry_instance(&self, _entity: &Entity) -> Option<Vec<u8>> {
        // DEVIATION: Requires full plane geometry construction
        None
    }

    /// Creates the geometry instance for outline.
    pub fn create_outline_geometry_instance(&self, _entity: &Entity) -> Option<Vec<u8>> {
        // DEVIATION: Requires full plane outline geometry construction
        None
    }
}

impl GeometryUpdater for PlaneGeometryUpdater {
    fn entity_id(&self) -> &str { &self.entity_id }
    fn fill_enabled(&self) -> bool { self.fill_enabled }
    fn outline_enabled(&self) -> bool { self.outline_enabled }
    fn is_on_surface(&self) -> bool { false }
    fn is_closed(&self) -> bool { true }
}
