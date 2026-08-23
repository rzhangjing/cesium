//! Ported from `packages/engine/Source/DataSources/PolylineVolumeGeometryUpdater.js`.

use crate::entity::Entity;
use crate::geometry_updater::GeometryUpdater;

/// A geometry updater that creates polyline_volume geometry instances from entity data.
pub struct polyline_volumeGeometryUpdater {
    entity_id: String,
    fill_enabled: bool,
    outline_enabled: bool,
}

impl polyline_volumeGeometryUpdater {
    /// Creates a new polyline_volume geometry updater.
    pub fn new(entity: &Entity) -> Self {
        Self {
            entity_id: entity.id.clone(),
            fill_enabled: true,
            outline_enabled: false,
        }
    }

    /// Creates the geometry instance for fill.
    pub fn create_fill_geometry_instance(&self, _entity: &Entity) -> Option<Vec<u8>> {
        // DEVIATION: Requires full polyline_volume geometry construction
        None
    }

    /// Creates the geometry instance for outline.
    pub fn create_outline_geometry_instance(&self, _entity: &Entity) -> Option<Vec<u8>> {
        // DEVIATION: Requires full polyline_volume outline geometry construction
        None
    }
}

impl GeometryUpdater for polyline_volumeGeometryUpdater {
    fn entity_id(&self) -> &str { &self.entity_id }
    fn fill_enabled(&self) -> bool { self.fill_enabled }
    fn outline_enabled(&self) -> bool { self.outline_enabled }
    fn is_on_surface(&self) -> bool { false }
    fn is_closed(&self) -> bool { true }
}
