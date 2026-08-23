//! Ported from `packages/engine/Source/DataSources/CompositeEntityCollection.js`.
//!
//! An entity collection that combines entities from multiple data sources.

use crate::entity::Entity;
use crate::entity_collection::EntityCollection;

/// An entity collection that combines entities from multiple data sources.
///
/// This is used internally by DataSourceDisplay to provide a unified
/// view of all entities across all loaded data sources.
///
/// In CesiumJS, CompositeEntityCollection implements the EntityCollection
/// interface by delegating to an ordered list of underlying EntityCollection
/// instances. When an entity is looked up, each collection is searched in
/// order until the entity is found.
pub struct CompositeEntityCollection {
    /// The ordered list of entity collections.
    collections: Vec<EntityCollection>,
    is_destroyed: bool,
}

impl CompositeEntityCollection {
    /// Creates a new composite entity collection.
    pub fn new() -> Self {
        Self {
            collections: Vec::new(),
            is_destroyed: false,
        }
    }

    /// Returns the number of entity collections.
    pub fn len(&self) -> usize {
        self.collections.len()
    }

    /// Returns whether the collection list is empty.
    pub fn is_empty(&self) -> bool {
        self.collections.is_empty()
    }

    /// Gets the entity collection at the given index.
    pub fn get_collection(&self, index: usize) -> Option<&EntityCollection> {
        self.collections.get(index)
    }

    /// Adds an entity collection to the beginning of the list.
    ///
    /// In CesiumJS, collections are inserted at index 0 so that
    /// the most recently added data source takes priority.
    pub fn add_collection(&mut self, collection: EntityCollection) {
        self.collections.insert(0, collection);
    }

    /// Removes an entity collection from the list.
    pub fn remove_collection(&mut self, index: usize) -> Option<EntityCollection> {
        if index < self.collections.len() {
            Some(self.collections.remove(index))
        } else {
            None
        }
    }

    /// Moves a collection from one index to another.
    pub fn move_collection(&mut self, from: usize, to: usize) {
        if from >= self.collections.len() || to >= self.collections.len() {
            return;
        }
        let collection = self.collections.remove(from);
        self.collections.insert(to, collection);
    }

    /// Returns the total number of entities across all collections.
    pub fn entity_count(&self) -> usize {
        self.collections.iter().map(|c| c.len()).sum()
    }

    /// Gets an entity by ID, searching collections in order.
    ///
    /// Returns the first entity found with the given ID.
    pub fn get_entity(&self, id: &str) -> Option<&Entity> {
        for collection in &self.collections {
            if let Some(entity) = collection.get(id) {
                return Some(entity);
            }
        }
        None
    }

    /// Returns whether any collection contains an entity with the given ID.
    pub fn contains(&self, id: &str) -> bool {
        self.collections.iter().any(|c| c.contains_entity(id))
    }

    /// Returns whether this composite collection has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this composite collection.
    pub fn destroy(&mut self) {
        self.collections.clear();
        self.is_destroyed = true;
    }
}

impl Default for CompositeEntityCollection {
    fn default() -> Self {
        Self::new()
    }
}
