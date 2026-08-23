//! Ported from `packages/engine/Source/DataSources/DataSourceDisplay.js`.
//!
//! Visualizes a collection of DataSource instances.

use crate::bounding_sphere_state::BoundingSphereState;
use crate::custom_data_source::CustomDataSource;
use crate::data_source::DataSource;
use crate::data_source_collection::DataSourceCollection;
use crate::entity::Entity;
use crate::visualizer::Visualizer;

/// Visualizes a collection of `DataSource` instances.
///
/// In CesiumJS, DataSourceDisplay is the central coordinator that:
/// 1. Listens to DataSourceCollection add/remove/move events
/// 2. Creates and manages visualizers for each data source
/// 3. Updates all visualizers each frame
/// 4. Provides `getBoundingSphere` for entity camera targeting
///
/// The `visualizers_callback` is a function that creates the array of
/// visualizers for each data source. By default, it creates all standard
/// visualizers (Billboard, Geometry, Label, Model, 3DTiles, Point, Path, Polyline).
pub struct DataSourceDisplay {
    /// The collection of data sources to display.
    data_sources: DataSourceCollection,
    /// The default data source for manually created entities.
    default_data_source: CustomDataSource,
    /// Whether all data sources are ready.
    ready: bool,
    is_destroyed: bool,
    /// Visualizers for the default data source.
    default_visualizers: Vec<Box<dyn Visualizer>>,
    /// Per-data-source visualizer sets (indexed by data source index).
    data_source_visualizers: Vec<Vec<Box<dyn Visualizer>>>,
}

impl DataSourceDisplay {
    /// Creates a new data source display.
    ///
    /// In CesiumJS, this takes `options.scene`, `options.dataSourceCollection`,
    /// and an optional `options.visualizersCallback`.
    ///
    /// DEVIATION: Scene is not stored directly; the caller is responsible
    /// for wiring scene primitives. The visualizers callback is simplified
    /// to a default set of visualizers.
    pub fn new(data_sources: DataSourceCollection) -> Self {
        let default_data_source = CustomDataSource::new("Default");
        Self {
            data_sources,
            default_data_source,
            ready: false,
            is_destroyed: false,
            default_visualizers: Vec::new(),
            data_source_visualizers: Vec::new(),
        }
    }

    /// Returns a reference to the data source collection.
    pub fn data_sources(&self) -> &DataSourceCollection {
        &self.data_sources
    }

    /// Returns a mutable reference to the data source collection.
    pub fn data_sources_mut(&mut self) -> &mut DataSourceCollection {
        &mut self.data_sources
    }

    /// Returns the default data source.
    ///
    /// This data source is always available and does not appear in the
    /// `dataSources` collection. It can be used to manually create and
    /// visualize entities not tied to a specific data source.
    pub fn default_data_source(&self) -> &CustomDataSource {
        &self.default_data_source
    }

    /// Returns a mutable reference to the default data source.
    pub fn default_data_source_mut(&mut self) -> &mut CustomDataSource {
        &mut self.default_data_source
    }

    /// Returns whether all data sources are ready.
    pub fn ready(&self) -> bool {
        self.ready
    }

    /// Sets the visualizers for the default data source.
    pub fn set_default_visualizers(&mut self, visualizers: Vec<Box<dyn Visualizer>>) {
        self.default_visualizers = visualizers;
    }

    /// Adds visualizers for a data source at the given index.
    pub fn set_visualizers(&mut self, index: usize, visualizers: Vec<Box<dyn Visualizer>>) {
        if index >= self.data_source_visualizers.len() {
            self.data_source_visualizers.resize_with(index + 1, Vec::new);
        }
        self.data_source_visualizers[index] = visualizers;
    }

    /// Updates the display to the given time.
    ///
    /// Iterates over all data sources and their visualizers, calling
    /// `update(time)` on each. Returns true if all data sources are
    /// ready to be displayed.
    ///
    /// In CesiumJS, this also calls `dataSource.update(time)` if defined,
    /// and requests a scene render when becoming ready.
    pub fn update(&mut self, time: f64) -> bool {
        if self.is_destroyed {
            return false;
        }

        let mut result = true;

        // Update each data source's visualizers
        let ds_count = self.data_sources.length();
        for i in 0..ds_count {
            if let Some(visualizers) = self.data_source_visualizers.get_mut(i) {
                for visualizer in visualizers.iter_mut() {
                    result = visualizer.update(time) && result;
                }
            }
        }

        // Update default data source visualizers
        for visualizer in self.default_visualizers.iter_mut() {
            result = visualizer.update(time) && result;
        }

        // Once ready, stay ready (to prevent entity update breaks)
        self.ready = self.ready || result;
        result
    }

    /// Computes a bounding sphere for the given entity.
    ///
    /// Searches through all data sources to find the one containing the entity,
    /// then queries each visualizer for its bounding sphere contribution.
    ///
    /// Returns `BoundingSphereState::Done` if the result is valid,
    /// `BoundingSphereState::Pending` if still computing, or
    /// `BoundingSphereState::Failed` if the entity has no visualization.
    pub fn get_bounding_sphere(
        &self,
        entity: &Entity,
        allow_partial: bool,
        result: &mut [f64; 4],
    ) -> BoundingSphereState {
        if !self.ready {
            return BoundingSphereState::Pending;
        }

        // Check default data source first
        let default_entities = self.default_data_source.entities();
        if default_entities.contains_entity(&entity.id) {
            return self.query_visualizers_for_bounding_sphere(
                &self.default_visualizers,
                entity,
                allow_partial,
                result,
            );
        }

        // Search data source collection
        let ds_count = self.data_sources.length();
        for i in 0..ds_count {
            if let Some(ds) = self.data_sources.get(i) {
                // DEVIATION: We need to check entity containment per data source.
                // Since DataSourceEntry doesn't hold entities directly, we check
                // the visualizers which implicitly know their entities.
                if let Some(visualizers) = self.data_source_visualizers.get(i) {
                    let state = self.query_visualizers_for_bounding_sphere(
                        visualizers, entity, allow_partial, result,
                    );
                    if state != BoundingSphereState::Failed {
                        return state;
                    }
                }
            }
        }

        BoundingSphereState::Failed
    }

    /// Queries visualizers for a bounding sphere.
    fn query_visualizers_for_bounding_sphere(
        &self,
        visualizers: &[Box<dyn Visualizer>],
        entity: &Entity,
        allow_partial: bool,
        result: &mut [f64; 4],
    ) -> BoundingSphereState {
        let mut count = 0u32;
        let mut scratch = [0.0f64; 4];

        for visualizer in visualizers {
            let state = visualizer.get_bounding_sphere(entity, &mut scratch);
            if !allow_partial && state == BoundingSphereState::Pending {
                return BoundingSphereState::Pending;
            }
            if state == BoundingSphereState::Done {
                // Accumulate into result (simplified: last wins)
                *result = scratch;
                count += 1;
            }
        }

        if count == 0 {
            BoundingSphereState::Failed
        } else {
            BoundingSphereState::Done
        }
    }

    /// Returns whether this display has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this display, releasing all visualizer resources.
    ///
    /// In CesiumJS, this also removes primitive collections from the scene
    /// and removes all event listeners.
    pub fn destroy(&mut self) {
        // Destroy all data source visualizers
        for visualizers in &mut self.data_source_visualizers {
            for visualizer in visualizers.iter_mut() {
                visualizer.destroy();
            }
        }
        self.data_source_visualizers.clear();

        // Destroy default visualizers
        for visualizer in &mut self.default_visualizers {
            visualizer.destroy();
        }
        self.default_visualizers.clear();

        self.is_destroyed = true;
    }
}

impl Default for DataSourceDisplay {
    fn default() -> Self {
        Self::new(DataSourceCollection::new())
    }
}
