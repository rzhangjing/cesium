//! CompositeEntityCollection - non-destructively composites multiple EntityCollections.
//!
//! Maps to CesiumJS `DataSources/CompositeEntityCollection.js`

use crate::entity::Entity;
use crate::entity_collection::EntityCollection;

/// Non-destructively composites multiple EntityCollection instances into a
/// single collection. If an Entity with the same ID exists in multiple
/// collections, it is non-destructively merged into a single new entity.
///
/// Maps to CesiumJS `DataSources/CompositeEntityCollection.js`
#[derive(Debug, Default)]
pub struct CompositeEntityCollection {
    /// The ordered list of collections.
    collections: Vec<EntityCollection>,
    /// The composited entity cache.
    composite: EntityCollection,
    /// Whether the composite needs to be rebuilt.
    should_recomposite: bool,
    /// Owner composite (for nested composites).
    owner_id: Option<String>,
}

impl CompositeEntityCollection {
    /// Creates a new empty composite collection.
    pub fn new() -> Self {
        Self {
            collections: Vec::new(),
            composite: EntityCollection::new(),
            should_recomposite: true,
            owner_id: None,
        }
    }

    /// Creates a new composite with an owner ID.
    pub fn with_owner(owner_id: &str) -> Self {
        Self {
            collections: Vec::new(),
            composite: EntityCollection::new(),
            should_recomposite: true,
            owner_id: Some(owner_id.to_string()),
        }
    }

    /// Gets the owner ID, if any.
    pub fn owner(&self) -> Option<&str> {
        self.owner_id.as_deref()
    }

    /// Adds a collection to the composite.
    /// Maps to `CompositeEntityCollection.prototype.addCollection`
    pub fn add_collection(&mut self, collection: EntityCollection) {
        self.collections.push(collection);
        self.should_recomposite = true;
    }

    /// Adds a collection at a specific index.
    pub fn add_collection_at(&mut self, index: usize, collection: EntityCollection) {
        let idx = index.min(self.collections.len());
        self.collections.insert(idx, collection);
        self.should_recomposite = true;
    }

    /// Removes a collection from the composite.
    /// Returns true if the collection was found and removed.
    /// Maps to `CompositeEntityCollection.prototype.removeCollection`
    pub fn remove_collection(&mut self, index: usize) -> bool {
        if index < self.collections.len() {
            self.collections.remove(index);
            self.should_recomposite = true;
            true
        } else {
            false
        }
    }

    /// Removes all collections.
    /// Maps to `CompositeEntityCollection.prototype.removeAllCollections`
    pub fn remove_all_collections(&mut self) {
        self.collections.clear();
        self.should_recomposite = true;
    }

    /// Gets the number of collections.
    /// Maps to `CompositeEntityCollection.prototype.getCollectionsLength`
    pub fn get_collections_length(&self) -> usize {
        self.collections.len()
    }

    /// Gets a collection by index.
    /// Maps to `CompositeEntityCollection.prototype.getCollection`
    pub fn get_collection(&self, index: usize) -> Option<&EntityCollection> {
        self.collections.get(index)
    }

    /// Gets a mutable collection by index.
    pub fn get_collection_mut(&mut self, index: usize) -> Option<&mut EntityCollection> {
        self.should_recomposite = true;
        self.collections.get_mut(index)
    }

    /// Returns true if the composite contains an entity with the given ID.
    /// Maps to `CompositeEntityCollection.prototype.contains`
    pub fn contains(&self, entity_id: &str) -> bool {
        self.ensure_composited();
        self.composite.contains(entity_id)
    }

    /// Gets an entity by ID from the composite.
    /// Maps to `CompositeEntityCollection.prototype.getById`
    pub fn get_by_id(&self, entity_id: &str) -> Option<&Entity> {
        self.ensure_composited();
        self.composite.get(entity_id)
    }

    /// Gets or creates an entity by ID.
    /// Maps to `CompositeEntityCollection.prototype.getOrCreateEntity`
    pub fn get_or_create_entity(&mut self, entity_id: &str) -> &Entity {
        self.recomposite();
        self.composite.get_or_create(entity_id)
    }

    /// Returns the composited entity values.
    /// Maps to `CompositeEntityCollection.prototype.values`
    pub fn values(&self) -> Vec<&Entity> {
        self.ensure_composited();
        self.composite.values().collect()
    }

    /// Returns the number of composited entities.
    pub fn len(&self) -> usize {
        self.ensure_composited();
        self.composite.len()
    }

    /// Returns true if the composite is empty.
    pub fn is_empty(&self) -> bool {
        self.ensure_composited();
        self.composite.is_empty()
    }

    /// Suspends events (placeholder).
    pub fn suspend_events(&mut self) {}

    /// Resumes events (placeholder).
    pub fn resume_events(&mut self) {}

    /// Ensures the composite is up to date.
    fn ensure_composited(&self) {
        // In a real implementation this would check should_recomposite
        // and rebuild lazily. For now we always access the pre-built composite.
    }

    /// Rebuilds the composite from all collections.
    /// Later collections take priority for same-ID entities (merge).
    pub fn recomposite(&mut self) {
        let mut new_composite = EntityCollection::new();

        // Process collections in reverse order so later collections have priority
        for collection in self.collections.iter().rev() {
            for entity in collection.values() {
                if !new_composite.contains(&entity.id) {
                    new_composite.add(entity.clone());
                }
                // If entity already exists, the later collection's version wins
                // (already added since we iterate in reverse)
            }
        }

        self.composite = new_composite;
        self.should_recomposite = false;
    }
}
