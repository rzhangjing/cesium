//! Ported from `packages/engine/Source/DataSources/GeoJsonDataSource.js`.
//!
//! A data source that loads GeoJSON files.

use cesium_core::event::Event;
use crate::data_source::DataSource;
use crate::entity_collection::EntityCollection;

/// A data source that loads GeoJSON files.
///
/// In CesiumJS, GeoJsonDataSource.js is ~1000 lines with full GeoJSON
/// parsing (Point, LineString, Polygon, Multi*, Feature, FeatureCollection)
/// and entity creation with styling.
pub struct GeoJsonDataSource {
    name: String,
    entity_collection: EntityCollection,
    is_loading: bool,
    is_destroyed: bool,
    show: bool,
    clamp_to_ground: bool,
    changed_event: Event,
    error_event: Event,
    loading_event: Event,
}

impl GeoJsonDataSource {
    /// Creates a new GeoJSON data source.
    pub fn new() -> Self {
        Self {
            name: String::from("GeoJSON"),
            entity_collection: EntityCollection::new(),
            is_loading: false,
            is_destroyed: false,
            show: true,
            clamp_to_ground: false,
            changed_event: Event::new(),
            error_event: Event::new(),
            loading_event: Event::new(),
        }
    }

    /// Loads GeoJSON from a JSON string.
    ///
    /// In CesiumJS, this parses the GeoJSON and creates entities for each
    /// Feature. Supports Point, MultiPoint, LineString, MultiLineString,
    /// Polygon, MultiPolygon, GeometryCollection, Feature, FeatureCollection.
    pub fn load_json(&mut self, _json: &str) -> bool {
        // DEVIATION: Requires full GeoJSON parsing (serde_json) and
        // coordinate transformation (lon/lat → Cartesian3).
        false
    }

    /// Loads GeoJSON from a URL.
    pub fn load_url(&mut self, _url: &str) -> bool {
        // DEVIATION: Requires HTTP fetch + load_json
        false
    }

    /// Sets whether to clamp entities to the ground.
    pub fn set_clamp_to_ground(&mut self, clamp: bool) {
        self.clamp_to_ground = clamp;
    }

    /// Returns whether entities are clamped to ground.
    pub fn clamp_to_ground(&self) -> bool {
        self.clamp_to_ground
    }

    /// Sets the name of this data source.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
}

impl Default for GeoJsonDataSource {
    fn default() -> Self { Self::new() }
}

impl DataSource for GeoJsonDataSource {
    fn name(&self) -> &str { &self.name }
    fn entities(&self) -> &EntityCollection { &self.entity_collection }
    fn is_loading(&self) -> bool { self.is_loading }
    fn is_destroyed(&self) -> bool { self.is_destroyed }
    fn changed_event(&self) -> &Event { &self.changed_event }
    fn error_event(&self) -> &Event { &self.error_event }
    fn loading_event(&self) -> &Event { &self.loading_event }
    fn show(&self) -> bool { self.show }
    fn set_show(&mut self, show: bool) { self.show = show; }
    fn destroy(&mut self) {
        self.entity_collection.destroy();
        self.is_destroyed = true;
    }
}
