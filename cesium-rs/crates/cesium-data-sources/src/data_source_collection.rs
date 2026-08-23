//! Ported from `packages/engine/Source/DataSources/DataSourceCollection.js`.
//!
//! A collection of data sources.

use cesium_core::event::Event;

/// A collection of data sources.
///
/// In CesiumJS, DataSourceCollection manages an ordered list of DataSource
/// instances and fires events when data sources are added, removed, or moved.
///
/// Properties:
/// - `length` → number of data sources
/// - `dataSourceAdded` (Event)
/// - `dataSourceRemoved` (Event)
/// - `dataSourceMoved` (Event)
pub struct DataSourceCollection {
    data_sources: Vec<DataSourceEntry>,
    /// Event fired when a data source is added. Args: (index).
    pub data_source_added: Event,
    /// Event fired when a data source is removed. Args: (index, data_source).
    pub data_source_removed: Event,
    /// Event fired when a data source is moved. Args: (data_source, new_index, old_index).
    pub data_source_moved: Event,
    is_destroyed: bool,
}

/// Internal wrapper for a data source entry.
///
/// In CesiumJS, each data source is a DataSource instance. Here we store
/// the name and a flag indicating the type, since Rust doesn't have
/// dynamic trait objects with interior mutability easily.
///
/// DEVIATION: Uses name-based tracking; full trait-object integration
/// requires the Viewer to wire actual DataSource implementations.
#[derive(Clone)]
pub struct DataSourceEntry {
    /// The name of this data source.
    pub name: String,
    /// Whether this data source is currently loading.
    pub is_loading: bool,
}

impl DataSourceCollection {
    /// Creates a new data source collection.
    pub fn new() -> Self {
        Self {
            data_sources: Vec::new(),
            data_source_added: Event::new(),
            data_source_removed: Event::new(),
            data_source_moved: Event::new(),
            is_destroyed: false,
        }
    }

    /// Returns the number of data sources in this collection.
    pub fn length(&self) -> usize {
        self.data_sources.len()
    }

    /// Returns whether this collection is empty.
    pub fn is_empty(&self) -> bool {
        self.data_sources.is_empty()
    }

    /// Gets the data source at the given index.
    pub fn get(&self, index: usize) -> Option<&DataSourceEntry> {
        self.data_sources.get(index)
    }

    /// Gets a mutable reference to the data source at the given index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut DataSourceEntry> {
        self.data_sources.get_mut(index)
    }

    /// Adds a data source to this collection.
    ///
    /// This will fire the `dataSourceAdded` event after the data source is added.
    pub fn add(&mut self, name: &str) -> usize {
        self.data_sources.push(DataSourceEntry {
            name: name.to_string(),
            is_loading: false,
        });
        let index = self.data_sources.len() - 1;
        // In CesiumJS, this fires dataSourceAdded(collection, dataSource)
        // DEVIATION: Event firing requires Arc<Mutex<>> listeners; placeholder
        index
    }

    /// Removes a data source from this collection.
    ///
    /// If `destroy` is true, the data source will also be destroyed.
    /// Returns the removed data source entry, if any.
    pub fn remove(&mut self, index: usize) -> Option<DataSourceEntry> {
        if index >= self.data_sources.len() {
            return None;
        }
        let entry = self.data_sources.remove(index);
        // In CesiumJS, this fires dataSourceRemoved(collection, dataSource)
        Some(entry)
    }

    /// Removes all data sources from this collection.
    pub fn remove_all(&mut self) {
        self.data_sources.clear();
    }

    /// Moves a data source from one index to another.
    pub fn move_entry(&mut self, from: usize, to: usize) {
        if from >= self.data_sources.len() || to >= self.data_sources.len() {
            return;
        }
        let entry = self.data_sources.remove(from);
        self.data_sources.insert(to, entry);
        // In CesiumJS, this fires dataSourceMoved(dataSource, newIndex, oldIndex)
    }

    /// Returns the index of the data source with the given name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.data_sources.iter().position(|e| e.name == name)
    }

    /// Returns whether this collection has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this collection and all data sources it contains.
    pub fn destroy(&mut self) {
        self.data_sources.clear();
        self.is_destroyed = true;
    }
}

impl Default for DataSourceCollection {
    fn default() -> Self {
        Self::new()
    }
}
