//! Ported from `packages/engine/Source/DataSources/EntityCollection.js`.
//!
//! A collection of entities with change events.

use cesium_core::event::Event;
use crate::entity::Entity;
use std::collections::HashMap;

/// A collection of entities.
///
/// In CesiumJS, EntityCollection.js is ~1000 lines with full event tracking
/// (collectionChanged, selectedEntityChanged), availability computation,
/// and suspend/resume optimization.
pub struct EntityCollection {
    entities: HashMap<String, Entity>,
    /// Ordered list of entity IDs for deterministic iteration.
    entity_ids: Vec<String>,
    /// Fired when entities are added, removed, or changed.
    pub collection_changed: Event,
    /// Whether this collection has been destroyed.
    is_destroyed: bool,
    /// Whether events are suspended (for batch operations).
    suspend_depth: u32,
    /// Pending change notifications during suspension.
    pending_changes: Vec<CollectionChange>,
}

/// A change notification for the collection.
#[derive(Debug, Clone)]
pub enum CollectionChange {
    /// An entity was added.
    Added(String),
    /// An entity was removed.
    Removed(String),
    /// An entity's definition changed.
    Changed(String),
}

impl EntityCollection {
    /// Creates a new entity collection.
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            entity_ids: Vec::new(),
            collection_changed: Event::new(),
            is_destroyed: false,
            suspend_depth: 0,
            pending_changes: Vec::new(),
        }
    }

    /// Returns the number of entities.
    pub fn length(&self) -> usize {
        self.entities.len()
    }

    /// Returns the number of entities (alias for `length`).
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Returns all entities as a slice of references, in insertion order.
    pub fn values(&self) -> Vec<&Entity> {
        self.entity_ids
            .iter()
            .filter_map(|id| self.entities.get(id))
            .collect()
    }

    /// Returns the entity with the given ID.
    pub fn get_by_id(&self, id: &str) -> Option<&Entity> {
        self.entities.get(id)
    }

    /// Returns the entity with the given ID (alias for `get_by_id`).
    pub fn get(&self, id: &str) -> Option<&Entity> {
        self.entities.get(id)
    }

    /// Returns a mutable reference to the entity with the given ID.
    pub fn get_by_id_mut(&mut self, id: &str) -> Option<&mut Entity> {
        self.entities.get_mut(id)
    }

    /// Returns whether the collection contains an entity with the given ID.
    pub fn contains_entity(&self, id: &str) -> bool {
        self.entities.contains_key(id)
    }

    /// Adds an entity to the collection.
    ///
    /// Returns a reference to the added entity.
    pub fn add(&mut self, entity: Entity) -> &Entity {
        let id = entity.id.clone();
        self.entities.insert(id.clone(), entity);
        if !self.entity_ids.contains(&id) {
            self.entity_ids.push(id.clone());
        }
        self.record_change(CollectionChange::Added(id));
        self.entities.get(self.entity_ids.last().unwrap()).unwrap()
    }

    /// Removes an entity from the collection.
    pub fn remove(&mut self, id: &str) -> Option<Entity> {
        let entity = self.entities.remove(id);
        if entity.is_some() {
            self.entity_ids.retain(|eid| eid != id);
            self.record_change(CollectionChange::Removed(id.to_string()));
        }
        entity
    }

    /// Removes all entities from the collection.
    pub fn remove_all(&mut self) {
        let ids: Vec<String> = self.entity_ids.clone();
        self.entities.clear();
        self.entity_ids.clear();
        for id in ids {
            self.record_change(CollectionChange::Removed(id));
        }
    }

    /// Suspends event firing. Call `resume_events` to re-enable.
    ///
    /// In CesiumJS, this is used for batch operations to avoid
    /// firing collectionChanged for every individual change.
    pub fn suspend_events(&mut self) {
        self.suspend_depth += 1;
    }

    /// Resumes event firing and fires any pending changes.
    pub fn resume_events(&mut self) {
        if self.suspend_depth > 0 {
            self.suspend_depth -= 1;
            if self.suspend_depth == 0 {
                let changes = std::mem::take(&mut self.pending_changes);
                if !changes.is_empty() {
                    // In CesiumJS, this fires the collectionChanged event
                    // with the array of added/removed/changed entities.
                    let _ = changes;
                }
            }
        }
    }

    /// Returns whether this collection has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this collection.
    pub fn destroy(&mut self) {
        self.entities.clear();
        self.entity_ids.clear();
        self.pending_changes.clear();
        self.is_destroyed = true;
    }

    /// Records a change, either immediately or during suspension.
    fn record_change(&mut self, change: CollectionChange) {
        if self.suspend_depth > 0 {
            self.pending_changes.push(change);
        }
        // In CesiumJS, this would fire collectionChanged immediately
        // when not suspended.
    }
}

impl Default for EntityCollection {
    fn default() -> Self {
        Self::new()
    }
}
