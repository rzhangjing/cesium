//! DataSourceCollection - a collection of DataSource instances.
//!
//! Maps to CesiumJS `DataSources/DataSourceCollection.js`

use crate::entity_collection::DataSource;

/// A collection of DataSource instances with ordering and event support.
///
/// Maps to CesiumJS `DataSources/DataSourceCollection.js`
#[derive(Debug, Default)]
pub struct DataSourceCollection {
    data_sources: Vec<DataSource>,
    destroyed: bool,
}

impl DataSourceCollection {
    /// Creates a new empty collection.
    pub fn new() -> Self {
        Self {
            data_sources: Vec::new(),
            destroyed: false,
        }
    }

    /// Gets the number of data sources in this collection.
    /// Maps to `DataSourceCollection.prototype.length`
    pub fn length(&self) -> usize {
        self.data_sources.len()
    }

    /// Adds a data source to the collection.
    /// Maps to `DataSourceCollection.prototype.add`
    pub fn add(&mut self, data_source: DataSource) {
        assert!(!self.destroyed, "This object was destroyed.");
        self.data_sources.push(data_source);
    }

    /// Inserts a data source at a specific index.
    pub fn insert(&mut self, index: usize, data_source: DataSource) {
        assert!(!self.destroyed, "This object was destroyed.");
        let idx = index.min(self.data_sources.len());
        self.data_sources.insert(idx, data_source);
    }

    /// Removes a data source from this collection, if present.
    /// Returns true if the data source was in the collection and was removed.
    /// Maps to `DataSourceCollection.prototype.remove`
    pub fn remove(&mut self, name: &str) -> bool {
        assert!(!self.destroyed, "This object was destroyed.");
        if let Some(index) = self.data_sources.iter().position(|ds| ds.name == name) {
            self.data_sources.remove(index);
            true
        } else {
            false
        }
    }

    /// Removes a data source by index.
    pub fn remove_at(&mut self, index: usize) -> Option<DataSource> {
        assert!(!self.destroyed, "This object was destroyed.");
        if index < self.data_sources.len() {
            Some(self.data_sources.remove(index))
        } else {
            None
        }
    }

    /// Removes all data sources from this collection.
    /// Maps to `DataSourceCollection.prototype.removeAll`
    pub fn remove_all(&mut self) {
        assert!(!self.destroyed, "This object was destroyed.");
        self.data_sources.clear();
    }

    /// Checks to see if the collection contains a given data source by name.
    /// Maps to `DataSourceCollection.prototype.contains`
    pub fn contains(&self, name: &str) -> bool {
        self.data_sources.iter().any(|ds| ds.name == name)
    }

    /// Determines the index of a given data source in the collection.
    /// Maps to `DataSourceCollection.prototype.indexOf`
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.data_sources.iter().position(|ds| ds.name == name)
    }

    /// Gets a data source by index from the collection.
    /// Maps to `DataSourceCollection.prototype.get`
    pub fn get(&self, index: usize) -> Option<&DataSource> {
        self.data_sources.get(index)
    }

    /// Gets a mutable data source by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut DataSource> {
        self.data_sources.get_mut(index)
    }

    /// Gets all data sources matching the provided name.
    /// Maps to `DataSourceCollection.prototype.getByName`
    pub fn get_by_name(&self, name: &str) -> Vec<&DataSource> {
        self.data_sources.iter().filter(|ds| ds.name == name).collect()
    }

    /// Raises a data source up one position in the collection.
    /// Maps to `DataSourceCollection.prototype.raise`
    pub fn raise(&mut self, name: &str) {
        let index = self
            .index_of(name)
            .expect("dataSource is not in this collection.");
        let len = self.data_sources.len();
        let new_index = (index + 1).min(len - 1);
        if index != new_index {
            self.data_sources.swap(index, new_index);
        }
    }

    /// Lowers a data source down one position in the collection.
    /// Maps to `DataSourceCollection.prototype.lower`
    pub fn lower(&mut self, name: &str) {
        let index = self
            .index_of(name)
            .expect("dataSource is not in this collection.");
        if index > 0 {
            self.data_sources.swap(index, index - 1);
        }
    }

    /// Raises a data source to the top of the collection.
    /// Maps to `DataSourceCollection.prototype.raiseToTop`
    pub fn raise_to_top(&mut self, name: &str) {
        let index = self
            .index_of(name)
            .expect("dataSource is not in this collection.");
        let len = self.data_sources.len();
        if index != len - 1 {
            let ds = self.data_sources.remove(index);
            self.data_sources.push(ds);
        }
    }

    /// Lowers a data source to the bottom of the collection.
    /// Maps to `DataSourceCollection.prototype.lowerToBottom`
    pub fn lower_to_bottom(&mut self, name: &str) {
        let index = self
            .index_of(name)
            .expect("dataSource is not in this collection.");
        if index != 0 {
            let ds = self.data_sources.remove(index);
            self.data_sources.insert(0, ds);
        }
    }

    /// Returns true if this object was destroyed.
    /// Maps to `DataSourceCollection.prototype.isDestroyed`
    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    /// Destroys the collection.
    /// Maps to `DataSourceCollection.prototype.destroy`
    pub fn destroy(&mut self) {
        self.data_sources.clear();
        self.destroyed = true;
    }

    /// Returns an iterator over the data sources.
    pub fn iter(&self) -> impl Iterator<Item = &DataSource> {
        self.data_sources.iter()
    }
}
