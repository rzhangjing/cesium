//! Ported from `packages/engine/Source/DataSources/GpxDataSource.js`.
//!
//! A data source that loads GPX (GPS Exchange Format) files.

use cesium_core::event::Event;
use crate::data_source::DataSource;
use crate::entity_collection::EntityCollection;

/// A data source that loads GPX (GPS Exchange Format) files.
///
/// In CesiumJS, GpxDataSource.js is ~400 lines with GPX XML parsing
/// that creates polyline entities from tracks and point entities from waypoints.
pub struct GpxDataSource {
    name: String,
    entity_collection: EntityCollection,
    is_loading: bool,
    is_destroyed: bool,
    show: bool,
    changed_event: Event,
    error_event: Event,
    loading_event: Event,
}

impl GpxDataSource {
    /// Creates a new GPX data source.
    pub fn new() -> Self {
        Self {
            name: String::from("GPX"),
            entity_collection: EntityCollection::new(),
            is_loading: false,
            is_destroyed: false,
            show: true,
            changed_event: Event::new(),
            error_event: Event::new(),
            loading_event: Event::new(),
        }
    }

    /// Loads GPX from an XML string.
    ///
    /// In CesiumJS, this parses the GPX XML and creates entities for
    /// waypoints (Point), tracks (Polyline), and routes (Polyline).
    pub fn load_xml(&mut self, _xml: &str) -> bool {
        // DEVIATION: Requires GPX XML parsing (quick-xml or similar)
        false
    }

    /// Loads a GPX file from the given URL.
    pub fn load_url(&mut self, _url: &str) -> bool {
        // DEVIATION: Requires HTTP fetch + load_xml
        false
    }

    /// Sets the name of this data source.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
}

impl Default for GpxDataSource {
    fn default() -> Self { Self::new() }
}

impl DataSource for GpxDataSource {
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
