//! Visualizer: manages entity-to-geometry mapping and batch processing.
//!
//! Maps to CesiumJS `DataSources/GeometryVisualizer.js`, `DataSources/Visualizer.js`
//!
//! The visualizer tracks entities from an EntityCollection, converts their
//! graphics properties into geometry instances, and manages static/dynamic batching.

use std::collections::HashMap;

use cesium_geospatial::Ellipsoid;

use crate::entity_collection::EntityCollection;
use crate::geometry_updater::{update_entity_geometry, EntityGeometry, GeometryInstance};

/// A visualizer that manages geometry generation for a collection of entities.
///
/// Maps to CesiumJS `DataSources/GeometryVisualizer.js`
#[derive(Debug)]
pub struct GeometryVisualizer {
    /// Cached geometry per entity ID.
    geometry_cache: HashMap<String, EntityGeometry>,
    /// Last update time.
    last_time: f64,
    /// The ellipsoid used for coordinate conversion.
    ellipsoid: Ellipsoid,
    /// Whether the visualizer needs a full rebuild.
    dirty: bool,
}

impl GeometryVisualizer {
    /// Creates a new geometry visualizer.
    pub fn new(ellipsoid: Ellipsoid) -> Self {
        Self {
            geometry_cache: HashMap::new(),
            last_time: 0.0,
            ellipsoid,
            dirty: true,
        }
    }

    /// Creates a new geometry visualizer with WGS84 ellipsoid.
    pub fn wgs84() -> Self {
        Self::new(Ellipsoid::WGS84)
    }

    /// Marks the visualizer as dirty (needs full rebuild).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Updates the visualizer for the given entity collection at the given time.
    ///
    /// Returns the number of entities that were updated.
    pub fn update(&mut self, entities: &EntityCollection, time: f64) -> usize {
        let time_changed = (time - self.last_time).abs() > f64::EPSILON;
        self.last_time = time;

        if !self.dirty && !time_changed {
            return 0;
        }

        let mut updated = 0;

        // Remove entities that no longer exist
        let current_ids: Vec<String> = entities.values().map(|e| e.id.clone()).collect();
        self.geometry_cache.retain(|id, _| current_ids.contains(id));

        // Update or add entities
        for entity in entities.values() {
            let needs_update = self.dirty
                || time_changed
                || !self.geometry_cache.contains_key(&entity.id);

            if needs_update {
                let geometry = update_entity_geometry(entity, time, &self.ellipsoid);
                self.geometry_cache.insert(entity.id.clone(), geometry);
                updated += 1;
            }
        }

        self.dirty = false;
        updated
    }

    /// Gets the geometry for a specific entity.
    pub fn get_geometry(&self, entity_id: &str) -> Option<&EntityGeometry> {
        self.geometry_cache.get(entity_id)
    }

    /// Returns all fill geometry instances across all entities.
    pub fn all_fill_instances(&self) -> Vec<&GeometryInstance> {
        self.geometry_cache
            .values()
            .flat_map(|g| g.fill_instances.iter())
            .collect()
    }

    /// Returns all outline geometry instances across all entities.
    pub fn all_outline_instances(&self) -> Vec<&GeometryInstance> {
        self.geometry_cache
            .values()
            .flat_map(|g| g.outline_instances.iter())
            .collect()
    }

    /// Returns all geometry instances (fill + outline).
    pub fn all_instances(&self) -> Vec<&GeometryInstance> {
        self.geometry_cache
            .values()
            .flat_map(|g| {
                g.fill_instances.iter().chain(g.outline_instances.iter())
            })
            .collect()
    }

    /// Total number of geometry instances.
    pub fn instance_count(&self) -> usize {
        self.geometry_cache
            .values()
            .map(|g| g.instance_count())
            .sum()
    }

    /// Number of entities being tracked.
    pub fn entity_count(&self) -> usize {
        self.geometry_cache.len()
    }

    /// Removes geometry for a specific entity.
    pub fn remove_entity(&mut self, entity_id: &str) {
        self.geometry_cache.remove(entity_id);
    }

    /// Clears all cached geometry.
    pub fn clear(&mut self) {
        self.geometry_cache.clear();
        self.dirty = true;
    }
}

/// A static geometry batch that combines multiple geometry instances
/// into a single batch for efficient rendering.
///
/// Maps to CesiumJS `DataSources/StaticGeometryColorBatch.js`
#[derive(Debug, Default)]
pub struct StaticGeometryBatch {
    /// Batched fill instances.
    pub fill_instances: Vec<GeometryInstance>,
    /// Batched outline instances.
    pub outline_instances: Vec<GeometryInstance>,
}

impl StaticGeometryBatch {
    /// Creates a new empty batch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds geometry instances to the batch.
    pub fn add(&mut self, geometry: &EntityGeometry) {
        self.fill_instances.extend(geometry.fill_instances.iter().cloned());
        self.outline_instances.extend(geometry.outline_instances.iter().cloned());
    }

    /// Total number of instances in the batch.
    pub fn len(&self) -> usize {
        self.fill_instances.len() + self.outline_instances.len()
    }

    /// Returns true if the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.fill_instances.is_empty() && self.outline_instances.is_empty()
    }

    /// Clears the batch.
    pub fn clear(&mut self) {
        self.fill_instances.clear();
        self.outline_instances.clear();
    }
}

/// A dynamic geometry updater that regenerates geometry each frame
/// for entities with time-dynamic properties.
///
/// Maps to CesiumJS `DataSources/DynamicGeometryUpdater.js`
#[derive(Debug)]
pub struct DynamicGeometryUpdater {
    /// Entity IDs that have dynamic (time-varying) geometry.
    dynamic_entities: Vec<String>,
    /// The ellipsoid used for coordinate conversion.
    ellipsoid: Ellipsoid,
}

impl DynamicGeometryUpdater {
    /// Creates a new dynamic geometry updater.
    pub fn new(ellipsoid: Ellipsoid) -> Self {
        Self {
            dynamic_entities: Vec::new(),
            ellipsoid,
        }
    }

    /// Registers an entity as dynamic.
    pub fn add_entity(&mut self, entity_id: &str) {
        if !self.dynamic_entities.contains(&entity_id.to_string()) {
            self.dynamic_entities.push(entity_id.to_string());
        }
    }

    /// Removes an entity from dynamic tracking.
    pub fn remove_entity(&mut self, entity_id: &str) {
        self.dynamic_entities.retain(|id| id != entity_id);
    }

    /// Updates dynamic geometry for all tracked entities at the given time.
    pub fn update(&self, entities: &EntityCollection, time: f64) -> Vec<(String, EntityGeometry)> {
        self.dynamic_entities
            .iter()
            .filter_map(|id| {
                entities.get(id).map(|entity| {
                    let geometry = update_entity_geometry(entity, time, &self.ellipsoid);
                    (id.clone(), geometry)
                })
            })
            .collect()
    }

    /// Number of dynamic entities being tracked.
    pub fn entity_count(&self) -> usize {
        self.dynamic_entities.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::*;
    use crate::property::Property;

    fn make_collection() -> EntityCollection {
        let mut collection = EntityCollection::new();
        collection.add(
            Entity::new("box-1")
                .with_position(0.0, 0.0, 0.0)
                .with_box(BoxGraphics {
                    dimensions: Property::Constant([100.0, 100.0, 100.0]),
                    ..Default::default()
                }),
        );
        collection.add(
            Entity::new("cyl-1")
                .with_position(0.1, 0.1, 0.0)
                .with_cylinder(CylinderGraphics {
                    length: Property::Constant(200.0),
                    top_radius: Property::Constant(50.0),
                    bottom_radius: Property::Constant(50.0),
                    ..Default::default()
                }),
        );
        collection
    }

    #[test]
    fn test_visualizer_update() {
        let mut viz = GeometryVisualizer::wgs84();
        let collection = make_collection();

        let updated = viz.update(&collection, 0.0);
        assert_eq!(updated, 2);
        assert_eq!(viz.entity_count(), 2);
        assert!(viz.instance_count() >= 2);
    }

    #[test]
    fn test_visualizer_no_change() {
        let mut viz = GeometryVisualizer::wgs84();
        let collection = make_collection();

        viz.update(&collection, 0.0);
        let updated = viz.update(&collection, 0.0);
        assert_eq!(updated, 0); // No change
    }

    #[test]
    fn test_visualizer_time_change() {
        let mut viz = GeometryVisualizer::wgs84();
        let collection = make_collection();

        viz.update(&collection, 0.0);
        let updated = viz.update(&collection, 1.0);
        assert_eq!(updated, 2); // Time changed, all updated
    }

    #[test]
    fn test_visualizer_entity_removal() {
        let mut viz = GeometryVisualizer::wgs84();
        let mut collection = make_collection();

        viz.update(&collection, 0.0);
        assert_eq!(viz.entity_count(), 2);

        collection.remove("box-1");
        viz.mark_dirty();
        viz.update(&collection, 0.0);
        assert_eq!(viz.entity_count(), 1);
    }

    #[test]
    fn test_visualizer_get_geometry() {
        let mut viz = GeometryVisualizer::wgs84();
        let collection = make_collection();

        viz.update(&collection, 0.0);
        let geo = viz.get_geometry("box-1").unwrap();
        assert_eq!(geo.fill_instances.len(), 1);
    }

    #[test]
    fn test_visualizer_all_instances() {
        let mut viz = GeometryVisualizer::wgs84();
        let collection = make_collection();

        viz.update(&collection, 0.0);
        let fills = viz.all_fill_instances();
        assert_eq!(fills.len(), 2);
    }

    #[test]
    fn test_static_batch() {
        let mut batch = StaticGeometryBatch::new();
        assert!(batch.is_empty());

        let geo = EntityGeometry {
            fill_instances: vec![],
            outline_instances: vec![],
        };
        batch.add(&geo);
        assert!(batch.is_empty());

        // Add real geometry
        let mut viz = GeometryVisualizer::wgs84();
        let collection = make_collection();
        viz.update(&collection, 0.0);

        for instance in viz.all_fill_instances() {
            batch.fill_instances.push(instance.clone());
        }
        assert!(!batch.is_empty());
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn test_dynamic_updater() {
        let mut dynamic = DynamicGeometryUpdater::new(Ellipsoid::WGS84);
        let collection = make_collection();

        dynamic.add_entity("box-1");
        assert_eq!(dynamic.entity_count(), 1);

        let results = dynamic.update(&collection, 0.0);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "box-1");
        assert_eq!(results[0].1.fill_instances.len(), 1);
    }

    #[test]
    fn test_dynamic_updater_remove() {
        let mut dynamic = DynamicGeometryUpdater::new(Ellipsoid::WGS84);
        dynamic.add_entity("box-1");
        dynamic.add_entity("cyl-1");
        assert_eq!(dynamic.entity_count(), 2);

        dynamic.remove_entity("box-1");
        assert_eq!(dynamic.entity_count(), 1);
    }

    #[test]
    fn test_visualizer_clear() {
        let mut viz = GeometryVisualizer::wgs84();
        let collection = make_collection();

        viz.update(&collection, 0.0);
        assert_eq!(viz.entity_count(), 2);

        viz.clear();
        assert_eq!(viz.entity_count(), 0);
        assert_eq!(viz.instance_count(), 0);
    }
}
