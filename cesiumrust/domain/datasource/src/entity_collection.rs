//! Entity collection management.
//!
//! Maps to CesiumJS `DataSources/EntityCollection.js`

use crate::entity::Entity;
use std::collections::HashMap;

/// A collection of entities with ID-based lookup.
///
/// Maps to CesiumJS `DataSources/EntityCollection.js`
#[derive(Debug, Default)]
pub struct EntityCollection {
    /// Entities indexed by ID.
    entities: HashMap<String, Entity>,

    /// Insertion order tracking.
    order: Vec<String>,

    /// Whether the collection is shown.
    show: bool,
}

impl EntityCollection {
    /// Creates a new empty collection.
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            order: Vec::new(),
            show: true,
        }
    }

    /// Adds or replaces an entity.
    pub fn add(&mut self, entity: Entity) {
        let id = entity.id.clone();
        if !self.entities.contains_key(&id) {
            self.order.push(id.clone());
        }
        self.entities.insert(id, entity);
    }

    /// Removes an entity by ID.
    pub fn remove(&mut self, id: &str) -> Option<Entity> {
        if let Some(entity) = self.entities.remove(id) {
            self.order.retain(|o| o != id);
            Some(entity)
        } else {
            None
        }
    }

    /// Gets an entity by ID.
    pub fn get(&self, id: &str) -> Option<&Entity> {
        self.entities.get(id)
    }

    /// Gets a mutable entity by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Entity> {
        self.entities.get_mut(id)
    }

    /// Returns the number of entities.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns true if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Returns true if the collection contains an entity with the given ID.
    pub fn contains(&self, id: &str) -> bool {
        self.entities.contains_key(id)
    }

    /// Clears all entities.
    pub fn clear(&mut self) {
        self.entities.clear();
        self.order.clear();
    }

    /// Returns entities in insertion order.
    pub fn values(&self) -> impl Iterator<Item = &Entity> {
        self.order.iter().filter_map(|id| self.entities.get(id))
    }

    /// Returns all entity IDs in insertion order.
    pub fn ids(&self) -> &[String] {
        &self.order
    }

    /// Returns whether the collection is shown.
    pub fn show(&self) -> bool {
        self.show
    }

    /// Sets whether the collection is shown.
    pub fn set_show(&mut self, show: bool) {
        self.show = show;
    }

    /// Returns only visible entities (show=true and collection show=true).
    pub fn visible_entities(&self) -> impl Iterator<Item = &Entity> {
        let show = self.show;
        self.values().filter(move |e| show && e.show)
    }

    /// Returns entities that have renderable graphics.
    pub fn renderable_entities(&self) -> impl Iterator<Item = &Entity> {
        self.visible_entities().filter(|e| e.has_graphics())
    }

    /// Suspends events (placeholder for future event system integration).
    pub fn suspend_events(&mut self) {
        // Placeholder
    }

    /// Resumes events.
    pub fn resume_events(&mut self) {
        // Placeholder
    }
}

/// A data source that provides entities.
///
/// Maps to CesiumJS `DataSources/DataSource.js`
#[derive(Debug)]
pub struct DataSource {
    /// The name of this data source.
    pub name: String,

    /// The entity collection.
    pub entities: EntityCollection,

    /// Whether the data source has been loaded.
    pub loaded: bool,

    /// Clock settings (if time-dynamic).
    pub clock_start: Option<f64>,
    pub clock_stop: Option<f64>,
    pub clock_current: Option<f64>,
}

impl DataSource {
    /// Creates a new data source with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entities: EntityCollection::new(),
            loaded: false,
            clock_start: None,
            clock_stop: None,
            clock_current: None,
        }
    }

    /// Returns true if this data source is ready for rendering.
    pub fn is_ready(&self) -> bool {
        self.loaded
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::PointGraphics;
    use crate::property::{Color, Property};

    #[test]
    fn test_entity_collection_add_get() {
        let mut collection = EntityCollection::new();
        collection.add(Entity::new("e1").with_name("Entity 1"));
        collection.add(Entity::new("e2").with_name("Entity 2"));

        assert_eq!(collection.len(), 2);
        assert!(collection.contains("e1"));
        assert!(collection.get("e2").is_some());
    }

    #[test]
    fn test_entity_collection_remove() {
        let mut collection = EntityCollection::new();
        collection.add(Entity::new("e1"));
        collection.add(Entity::new("e2"));

        let removed = collection.remove("e1");
        assert!(removed.is_some());
        assert_eq!(collection.len(), 1);
        assert!(!collection.contains("e1"));
    }

    #[test]
    fn test_entity_collection_order() {
        let mut collection = EntityCollection::new();
        collection.add(Entity::new("a"));
        collection.add(Entity::new("b"));
        collection.add(Entity::new("c"));

        let ids: Vec<&str> = collection.values().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_entity_collection_clear() {
        let mut collection = EntityCollection::new();
        collection.add(Entity::new("e1"));
        collection.add(Entity::new("e2"));

        collection.clear();
        assert!(collection.is_empty());
    }

    #[test]
    fn test_visible_entities() {
        let mut collection = EntityCollection::new();

        let mut visible = Entity::new("v1");
        visible.show = true;
        visible.point = Some(PointGraphics::default());
        collection.add(visible);

        let mut hidden = Entity::new("h1");
        hidden.show = false;
        hidden.point = Some(PointGraphics::default());
        collection.add(hidden);

        assert_eq!(collection.visible_entities().count(), 1);
    }

    #[test]
    fn test_renderable_entities() {
        let mut collection = EntityCollection::new();

        // Has graphics
        let with_gfx = Entity::new("gfx")
            .with_point(PointGraphics {
                color: Property::Constant(Color::RED),
                ..Default::default()
            });
        collection.add(with_gfx);

        // No graphics
        collection.add(Entity::new("no-gfx"));

        assert_eq!(collection.renderable_entities().count(), 1);
    }

    #[test]
    fn test_data_source() {
        let mut ds = DataSource::new("Test Source");
        assert!(!ds.is_ready());

        ds.loaded = true;
        assert!(ds.is_ready());

        ds.entities.add(Entity::new("e1"));
        assert_eq!(ds.entities.len(), 1);
    }
}
