//! Ported from `packages/engine/Source/DataSources/CustomDataSource.js`.
//!
//! A data source that can be populated with custom entities.

use cesium_core::event::Event;
use crate::data_source::DataSource;
use crate::entity_collection::EntityCollection;

/// A data source that can be populated with custom entities.
///
/// In CesiumJS, CustomDataSource is the simplest DataSource implementation.
/// It provides an EntityCollection that users can populate directly, and
/// is used as the `defaultDataSource` in DataSourceDisplay.
///
/// Each data source also has associated primitive collections and visualizers
/// that are managed by DataSourceDisplay.
pub struct CustomDataSource {
    /// The name of this data source.
    pub name: String,
    /// The entities in this data source.
    entities: EntityCollection,
    is_loading: bool,
    is_destroyed: bool,
    show: bool,
    changed_event: Event,
    error_event: Event,
    loading_event: Event,
}

impl CustomDataSource {
    /// Creates a new custom data source.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            entities: EntityCollection::new(),
            is_loading: false,
            is_destroyed: false,
            show: true,
            changed_event: Event::new(),
            error_event: Event::new(),
            loading_event: Event::new(),
        }
    }

    /// Returns a mutable reference to the entities.
    pub fn entities_mut(&mut self) -> &mut EntityCollection {
        &mut self.entities
    }
}

impl DataSource for CustomDataSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn entities(&self) -> &EntityCollection {
        &self.entities
    }

    fn is_loading(&self) -> bool {
        self.is_loading
    }

    fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    fn changed_event(&self) -> &Event {
        &self.changed_event
    }

    fn error_event(&self) -> &Event {
        &self.error_event
    }

    fn loading_event(&self) -> &Event {
        &self.loading_event
    }

    fn show(&self) -> bool {
        self.show
    }

    fn set_show(&mut self, show: bool) {
        self.show = show;
    }

    fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

impl Default for CustomDataSource {
    fn default() -> Self {
        Self::new("CustomDataSource")
    }
}
