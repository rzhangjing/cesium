//! Ported from `packages/engine/Source/DataSources/EntityCollection.js`.
//!
//! A collection of entities with change events.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use cesium_core::create_guid::create_guid;
use cesium_core::developer_error::throw_developer_error;
use cesium_core::event::{Event, ListenerId};
use cesium_core::iso8601::Iso8601;
use cesium_core::julian_date::JulianDate;
use cesium_core::time_interval::TimeInterval;

use crate::entity::{Entity, EntityDefinitionChangedArgs};
use crate::property::PropertyResult;

/// The payload of [`EntityCollection::collection_changed`].
///
/// Port of the `EntityCollection.CollectionChangedEventCallback` argument
/// list. DEVIATION: CesiumJS passes the collection itself plus arrays of
/// the affected `Entity` instances; the Rust port passes the entity ids
/// (entities are owned by the collection, so shared references cannot
/// cross the listener boundary without an `Rc` re-architecture).
/// See docs/deviations.md.
#[derive(Debug, Clone, Default)]
pub struct CollectionChangedArgs {
    /// The ids of the entities that have been added to the collection.
    pub added: Vec<String>,
    /// The ids of the entities that have been removed from the collection.
    pub removed: Vec<String>,
    /// The ids of the entities that have been modified.
    pub changed: Vec<String>,
}

/// Shared bookkeeping state for the `collectionChanged` event machinery.
///
/// Port of the CesiumJS private fields `_addedEntities` / `_removedEntities`
/// / `_changedEntities` / `_suspendCount` / `_firing` / `_refire`. The state
/// is shared through an `Rc` so that the per-entity `definitionChanged`
/// subscriptions registered on `add` can record changes and fire the event
/// without borrowing the collection (the entities are owned by it).
struct CollectionEventState {
    suspend_count: Cell<u32>,
    firing: Cell<bool>,
    refire: Cell<bool>,
    added_entities: RefCell<Vec<String>>,
    removed_entities: RefCell<Vec<String>>,
    changed_entities: RefCell<Vec<String>>,
    collection_changed: Event<CollectionChangedArgs>,
}

impl CollectionEventState {
    fn new() -> Self {
        Self {
            suspend_count: Cell::new(0),
            firing: Cell::new(false),
            refire: Cell::new(false),
            added_entities: RefCell::new(Vec::new()),
            removed_entities: RefCell::new(Vec::new()),
            changed_entities: RefCell::new(Vec::new()),
            collection_changed: Event::new(),
        }
    }
}

/// AssociativeArray.set stand-in: insert `id` keeping first-insertion order
/// and without duplicates.
fn set_id(list: &mut Vec<String>, id: &str) {
    if !list.iter().any(|entry| entry == id) {
        list.push(id.to_string());
    }
}

/// AssociativeArray.remove stand-in: remove `id`, returning whether it was
/// present.
fn remove_id(list: &mut Vec<String>, id: &str) -> bool {
    if let Some(position) = list.iter().position(|entry| entry == id) {
        list.remove(position);
        true
    } else {
        false
    }
}

fn take_ids(list: &RefCell<Vec<String>>) -> Vec<String> {
    std::mem::take(&mut *list.borrow_mut())
}

/// Port of the module-level `fireChangedEvent(collection)` helper.
fn fire_changed_event(state: &CollectionEventState) {
    if state.firing.get() {
        state.refire.set(true);
        return;
    }

    if state.suspend_count.get() == 0 {
        let has_changes = !state.added_entities.borrow().is_empty()
            || !state.removed_entities.borrow().is_empty()
            || !state.changed_entities.borrow().is_empty();
        if has_changes {
            state.firing.set(true);
            loop {
                state.refire.set(false);
                let added = take_ids(&state.added_entities);
                let removed = take_ids(&state.removed_entities);
                let changed = take_ids(&state.changed_entities);

                state.collection_changed.raise_event(&CollectionChangedArgs {
                    added,
                    removed,
                    changed,
                });
                if !state.refire.get() {
                    break;
                }
            }
            state.firing.set(false);
        }
    }
}

/// Port of `EntityCollection.prototype._onEntityDefinitionChanged`
/// (free function: the collection cannot be captured by the listener
/// closure because it owns the entities).
fn on_entity_definition_changed(state: &CollectionEventState, id: &str) {
    if !state.added_entities.borrow().iter().any(|entry| entry == id) {
        set_id(&mut state.changed_entities.borrow_mut(), id);
    }
    fire_changed_event(state);
}

/// A collection of entities.
///
/// In CesiumJS, EntityCollection.js is ~440 lines with full event tracking
/// (collectionChanged, suspend/resume batching, entity definition-change
/// bubbling), availability computation, and id-based bookkeeping.
pub struct EntityCollection {
    entities: HashMap<String, Entity>,
    /// Ordered list of entity IDs for deterministic iteration.
    entity_ids: Vec<String>,
    /// Shared `collectionChanged` bookkeeping (shared with the per-entity
    /// `definitionChanged` subscriptions).
    state: Rc<CollectionEventState>,
    /// Listener handles of the per-entity `definitionChanged`
    /// subscriptions, keyed by entity id (port of the
    /// `_onEntityDefinitionChanged` listener registration).
    entity_listeners: HashMap<String, ListenerId>,
    /// A globally unique identifier for this collection (port of `_id`,
    /// initialized with `createGuid()`).
    id: String,
    /// Whether the entities in this collection are shown.
    ///
    /// DEVIATION: the field stays public for the existing pipeline code;
    /// prefer [`EntityCollection::set_show`], which mirrors the JS setter's
    /// `isShowing` notification chain (direct field writes bypass it).
    /// See docs/deviations.md.
    pub show: bool,
    /// Whether this collection has been destroyed.
    is_destroyed: bool,
    // DEVIATION: the JS `owner` getter returns the DataSource or
    // CompositeEntityCollection that created this collection; the Rust
    // value model omits the back-reference (it would create an ownership
    // cycle with the owning data source). See docs/deviations.md.
}

impl EntityCollection {
    /// Creates a new entity collection.
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            entity_ids: Vec::new(),
            state: Rc::new(CollectionEventState::new()),
            entity_listeners: HashMap::new(),
            id: create_guid(),
            show: true,
            is_destroyed: false,
        }
    }

    /// Gets a globally unique identifier for this collection (port of the
    /// `id` getter; the value is created with `createGuid()` at
    /// construction time).
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Gets the event that is fired when entities are added or removed from
    /// the collection (port of the `collectionChanged` getter).
    ///
    /// The generated event is a [`CollectionChangedArgs`] payload.
    pub fn collection_changed(&self) -> &Event<CollectionChangedArgs> {
        &self.state.collection_changed
    }

    /// Returns the number of entities.
    pub fn length(&self) -> usize {
        self.entities.len()
    }

    /// Sets whether the entities in this collection are shown.
    ///
    /// Port of the `show` setter: when the value changes, the entity
    /// `isShowing` state is sampled before and after the assignment and a
    /// `definitionChanged("isShowing")` event is raised for every entity
    /// whose computed showing state changed, batched through
    /// `suspendEvents`/`resumeEvents`.
    ///
    /// DEVIATION: the JS `Entity.isShowing` getter folds the parent chain
    /// into the computation; the Rust value model has no entity hierarchy,
    /// so `isShowing` reduces to `entity.show && collection.show`. See
    /// docs/deviations.md.
    pub fn set_show(&mut self, value: bool) {
        // JS: `if (!defined(value)) throw DeveloperError` — statically
        // guaranteed in Rust.
        if value == self.show {
            return;
        }

        // Since entity.isShowing includes the EntityCollection.show state
        // in its calculation, loop over the entities twice: once to get the
        // old showing value and a second time to raise the changed event.
        self.suspend_events();

        let mut old_shows: Vec<(String, bool)> = Vec::new();
        for id in &self.entity_ids {
            if let Some(entity) = self.entities.get(id) {
                old_shows.push((id.clone(), entity.show && self.show));
            }
        }

        self.show = value;

        for (id, old_show) in old_shows {
            let Some(entity) = self.entities.get(&id) else {
                continue;
            };
            let is_showing = entity.show && self.show;
            if old_show != is_showing {
                entity.definition_changed.raise_event(&EntityDefinitionChangedArgs {
                    property_name: "isShowing".to_string(),
                    new_value: PropertyResult::Boolean(is_showing),
                    old_value: PropertyResult::Boolean(old_show),
                });
            }
        }

        self.resume_events();
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
    ///
    /// DEVIATION: CesiumJS observes entity mutations through the
    /// `definitionChanged` subscription installed on `add`; mutations made
    /// through the returned reference only reach the collection when they
    /// go through the [`Entity`] `set_*` mutators (which raise the event).
    /// See docs/deviations.md.
    pub fn get_by_id_mut(&mut self, id: &str) -> Option<&mut Entity> {
        self.entities.get_mut(id)
    }

    /// Returns the entity with the given ID, creating it if it does not
    /// exist (mirror of the internal `getOrCreateEntity` helper used by the
    /// CZML processing pipeline).
    pub fn get_or_create_entity(&mut self, id: &str) -> &mut Entity {
        if !self.entities.contains_key(id) {
            self.add(Entity::new(id));
        }
        self.entities.get_mut(id).unwrap()
    }

    /// Computes the union of the availability of all entities.
    ///
    /// Port of `EntityCollection.computeAvailability`: the result spans from
    /// the earliest entity availability start to the latest stop. When no
    /// entity defines availability, the result spans `Iso8601.MINIMUM_VALUE`
    /// to `Iso8601.MAXIMUM_VALUE`.
    pub fn compute_availability(&self) -> TimeInterval {
        // Mirrors `EntityCollection.computeAvailability`: reduce the entity
        // availability intervals with the empty interval as identity.
        let mut result: Option<TimeInterval> = None;
        for id in &self.entity_ids {
            let Some(entity) = self.entities.get(id) else { continue };
            for interval in &entity.availability {
                result = Some(match result {
                    None => interval.clone(),
                    Some(acc) => {
                        // Union of two intervals (outer span); the CZML
                        // availability intervals only feed the clock
                        // derivation so the boundary inclusion flags are
                        // taken from the accumulated span.
                        TimeInterval::new(
                            Some(if JulianDate::compare(&interval.start, &acc.start) < 0 {
                                interval.start.clone()
                            } else {
                                acc.start.clone()
                            }),
                            Some(if JulianDate::compare(&interval.stop, &acc.stop) > 0 {
                                interval.stop.clone()
                            } else {
                                acc.stop.clone()
                            }),
                            None,
                            None,
                        )
                    }
                });
            }
        }
        result.unwrap_or_else(|| TimeInterval::new(
            Some(Iso8601::minimum_value().clone()),
            Some(Iso8601::maximum_value().clone()),
            None,
            None,
        ))
    }

    /// Returns whether the collection contains an entity with the given ID.
    pub fn contains_entity(&self, id: &str) -> bool {
        self.entities.contains_key(id)
    }

    /// Adds an entity to the collection.
    ///
    /// Port of `EntityCollection.prototype.add`: records the addition,
    /// subscribes to the entity's `definitionChanged` event, and fires
    /// `collectionChanged`. Returns a reference to the added entity.
    ///
    /// # Panics
    ///
    /// `DeveloperError` if an entity with the same id already exists
    /// (the CesiumJS check is not debug-gated, so neither is this one).
    pub fn add(&mut self, entity: Entity) -> &Entity {
        // >>includeStart('debug', pragmas.debug)
        #[cfg(debug_assertions)]
        {
            // JS: Check `entity is required.` — statically guaranteed.
        }
        // >>includeEnd('debug');

        let id = entity.id.clone();
        if self.entities.contains_key(&id) {
            throw_developer_error(&format!(
                "An entity with id {id} already exists in this collection."
            ));
        }

        // Subscribe to the entity's definitionChanged event before moving
        // it into the collection (port of
        // `entity.definitionChanged.addEventListener(
        //     EntityCollection.prototype._onEntityDefinitionChanged, this)`).
        let state = Rc::clone(&self.state);
        let listener_entity_id = id.clone();
        let listener_id = entity.definition_changed.add_listener(move |_args| {
            on_entity_definition_changed(&state, &listener_entity_id);
        });
        self.entity_listeners.insert(id.clone(), listener_id.id());

        self.entities.insert(id.clone(), entity);
        self.entity_ids.push(id.clone());

        // Port of: `if (!this._removedEntities.remove(id)) {
        //               this._addedEntities.set(id, entity); }`
        if !remove_id(&mut self.state.removed_entities.borrow_mut(), &id) {
            set_id(&mut self.state.added_entities.borrow_mut(), &id);
        }

        fire_changed_event(&self.state);
        self.entities.get(&id).unwrap()
    }

    /// Removes an entity from the collection.
    ///
    /// Port of `EntityCollection.prototype.remove` / `removeById` (the Rust
    /// API removes by id and returns the removed entity).
    pub fn remove(&mut self, id: &str) -> Option<Entity> {
        let Some(entity) = self.entities.remove(id) else {
            return None;
        };
        self.entity_ids.retain(|eid| eid != id);

        // Unsubscribe from the entity's definitionChanged event (port of
        // `entity.definitionChanged.removeEventListener(...)`).
        if let Some(listener_id) = self.entity_listeners.remove(id) {
            entity.definition_changed.remove_listener(listener_id);
        }

        // Port of: `if (!this._addedEntities.remove(id)) {
        //               this._removedEntities.set(id, entity);
        //               this._changedEntities.remove(id); }`
        if !remove_id(&mut self.state.added_entities.borrow_mut(), id) {
            set_id(&mut self.state.removed_entities.borrow_mut(), id);
            remove_id(&mut self.state.changed_entities.borrow_mut(), id);
        }

        fire_changed_event(&self.state);
        Some(entity)
    }

    /// Removes all entities from the collection.
    ///
    /// Port of `EntityCollection.prototype.removeAll`: the event only
    /// contains items added before events were suspended plus the current
    /// contents of the collection.
    pub fn remove_all(&mut self) {
        let ids: Vec<String> = self.entity_ids.clone();
        for id in &ids {
            // Entities added during the current pending window are dropped
            // from the event entirely (mirrors the `addedItem` check).
            if !self
                .state
                .added_entities
                .borrow()
                .iter()
                .any(|entry| entry == id)
            {
                if let Some(entity) = self.entities.get(id) {
                    if let Some(listener_id) = self.entity_listeners.remove(id) {
                        entity.definition_changed.remove_listener(listener_id);
                    }
                }
                set_id(&mut self.state.removed_entities.borrow_mut(), id);
            } else {
                self.entity_listeners.remove(id);
            }
        }

        self.entities.clear();
        self.entity_ids.clear();
        self.state.added_entities.borrow_mut().clear();
        self.state.changed_entities.borrow_mut().clear();
        fire_changed_event(&self.state);
    }

    /// Prevents `collectionChanged` events from being raised until a
    /// corresponding call is made to [`EntityCollection::resume_events`],
    /// at which point a single event will be raised that covers all
    /// suspended operations.
    ///
    /// Port of `EntityCollection.prototype.suspendEvents`; reference
    /// counted, safe to call multiple times.
    pub fn suspend_events(&mut self) {
        self.state
            .suspend_count
            .set(self.state.suspend_count.get() + 1);
    }

    /// Resumes raising `collectionChanged` events immediately when an item
    /// is added or removed. Any modifications made while events were
    /// suspended are triggered as a single event when this function is
    /// called.
    ///
    /// Port of `EntityCollection.prototype.resumeEvents`.
    ///
    /// # Panics
    ///
    /// `DeveloperError` if called before `suspend_events` (debug-gated, as
    /// in CesiumJS).
    pub fn resume_events(&mut self) {
        // >>includeStart('debug', pragmas.debug);
        #[cfg(debug_assertions)]
        {
            if self.state.suspend_count.get() == 0 {
                throw_developer_error("resumeEvents can not be called before suspendEvents.");
            }
        }
        // >>includeEnd('debug');

        self.state
            .suspend_count
            .set(self.state.suspend_count.get() - 1);
        fire_changed_event(&self.state);
    }

    /// Returns whether this collection has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this collection.
    pub fn destroy(&mut self) {
        self.entities.clear();
        self.entity_ids.clear();
        self.entity_listeners.clear();
        self.state.added_entities.borrow_mut().clear();
        self.state.removed_entities.borrow_mut().clear();
        self.state.changed_entities.borrow_mut().clear();
        self.is_destroyed = true;
    }
}

impl Default for EntityCollection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    // Mirrors EntityCollectionSpec "id" — a new collection exposes a
    // globally unique identifier.
    #[test]
    fn entity_collection_id_is_unique_guid() {
        let a = EntityCollection::new();
        let b = EntityCollection::new();
        assert!(!a.id().is_empty());
        assert_ne!(a.id(), b.id());
    }

    // Mirrors EntityCollectionSpec show-setter behavior: setting the same
    // value is a no-op; a changed value raises `definitionChanged`
    // ("isShowing") for entities whose computed showing state changed.
    #[test]
    fn entity_collection_set_show_raises_is_showing() {
        let mut collection = EntityCollection::new();
        collection.add(Entity::new("a"));

        let fired = Rc::new(Cell::new(0usize));

        // Attach directly to the stored entity's definitionChanged.
        let listener_id = {
            let entity = collection.get_by_id("a").unwrap();
            let fired_clone = Rc::clone(&fired);
            entity.definition_changed.add_listener(move |args| {
                if args.property_name == "isShowing" {
                    fired_clone.set(fired_clone.get() + 1);
                    assert_eq!(args.new_value.as_bool(), Some(false));
                    assert_eq!(args.old_value.as_bool(), Some(true));
                }
            })
        };
        let _ = listener_id;

        // Same value: no event.
        collection.set_show(true);
        assert_eq!(fired.get(), 0);

        // true -> false: entity isShowing flips true -> false.
        collection.set_show(false);
        assert_eq!(fired.get(), 1);
        assert!(!collection.show);

        // An entity with show = false does not flip when the collection
        // toggles back on (isShowing stays false).
        collection.get_by_id_mut("a").unwrap().show = false;
        collection.set_show(true);
        assert_eq!(fired.get(), 1);
    }
}
