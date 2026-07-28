//! CustomDataSource - a basic named DataSource with an entity collection.
//!
//! Maps to CesiumJS `DataSources/CustomDataSource.js`

use crate::datasource_clock::DataSourceClock;
use crate::entity_collection::EntityCollection;

/// A basic DataSource with a name, entity collection, clock, and visibility.
///
/// Maps to CesiumJS `DataSources/CustomDataSource.js`
#[derive(Debug)]
pub struct CustomDataSource {
    /// The display name of this data source.
    name: String,
    /// The collection of entities.
    entities: EntityCollection,
    /// The clock associated with this data source.
    clock: Option<DataSourceClock>,
    /// Whether the data source is currently shown.
    show: bool,
    /// Whether the data source is currently loading.
    is_loading: bool,
}

impl CustomDataSource {
    /// Creates a new CustomDataSource with the given name.
    ///
    /// Maps to `new CustomDataSource(name)`
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            entities: EntityCollection::new(),
            clock: None,
            show: true,
            is_loading: false,
        }
    }

    /// Gets the name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the name.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    /// Gets the entity collection.
    pub fn entities(&self) -> &EntityCollection {
        &self.entities
    }

    /// Gets the entity collection mutably.
    pub fn entities_mut(&mut self) -> &mut EntityCollection {
        &mut self.entities
    }

    /// Gets the clock.
    pub fn clock(&self) -> Option<&DataSourceClock> {
        self.clock.as_ref()
    }

    /// Sets the clock.
    pub fn set_clock(&mut self, clock: Option<DataSourceClock>) {
        self.clock = clock;
    }

    /// Gets whether the data source is shown.
    pub fn show(&self) -> bool {
        self.show
    }

    /// Sets whether the data source is shown.
    pub fn set_show(&mut self, show: bool) {
        self.show = show;
        self.entities.set_show(show);
    }

    /// Gets whether the data source is loading.
    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    /// Sets whether the data source is loading.
    pub fn set_is_loading(&mut self, is_loading: bool) {
        self.is_loading = is_loading;
    }
}
