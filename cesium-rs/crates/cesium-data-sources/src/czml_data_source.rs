//! Ported from `packages/engine/Source/DataSources/CzmlDataSource.js`.
//!
//! A data source that loads CZML (Cesium Language) files.
//! CZML is a JSON-based format for describing time-dynamic 3D scenes.

use cesium_core::event::Event;
use crate::data_source::DataSource;
use crate::entity_collection::EntityCollection;

/// A data source that loads CZML (Cesium Language) files.
///
/// In CesiumJS, CzmlDataSource.js is ~1500 lines with full CZML packet
/// processing, time-dynamic property evaluation, and entity creation.
pub struct CzmlDataSource {
    name: String,
    entity_collection: EntityCollection,
    is_loading: bool,
    is_destroyed: bool,
    show: bool,
    changed_event: Event,
    error_event: Event,
    loading_event: Event,
}

impl CzmlDataSource {
    /// Creates a new CZML data source.
    pub fn new() -> Self {
        Self {
            name: String::from("CZML"),
            entity_collection: EntityCollection::new(),
            is_loading: false,
            is_destroyed: false,
            show: true,
            changed_event: Event::new(),
            error_event: Event::new(),
            loading_event: Event::new(),
        }
    }

    /// Loads CZML from a JSON string.
    ///
    /// In CesiumJS, this parses the CZML JSON array, processes each packet,
    /// and creates/updates entities accordingly.
    pub fn load_json(&mut self, _json: &str) -> bool {
        // DEVIATION: Requires full CZML JSON parsing (serde_json) and
        // packet processing (document, position, billboard, label, etc.)
        false
    }

    /// Loads CZML from a URL.
    pub fn load_url(&mut self, _url: &str) -> bool {
        // DEVIATION: Requires HTTP fetch + load_json
        false
    }

    /// Sets the name of this data source.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
}

impl Default for CzmlDataSource {
    fn default() -> Self { Self::new() }
}

impl DataSource for CzmlDataSource {
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
