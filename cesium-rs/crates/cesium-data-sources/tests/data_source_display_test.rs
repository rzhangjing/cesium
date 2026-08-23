//! Integration tests for DataSourceDisplay, DataSourceCollection, and related types.

use cesium_data_sources::composite_entity_collection::CompositeEntityCollection;
use cesium_data_sources::custom_data_source::CustomDataSource;
use cesium_data_sources::data_source::DataSource;
use cesium_data_sources::data_source_collection::{DataSourceCollection, DataSourceEntry};
use cesium_data_sources::data_source_display::DataSourceDisplay;
use cesium_data_sources::entity::Entity;
use cesium_data_sources::entity_collection::EntityCollection;

#[test]
fn test_entity_collection_add_and_get() {
    let mut collection = EntityCollection::new();
    assert_eq!(collection.len(), 0);
    assert!(collection.is_empty());

    let entity = Entity::new("entity-1");
    collection.add(entity);
    assert_eq!(collection.len(), 1);
    assert!(!collection.is_empty());

    let retrieved = collection.get("entity-1");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, "entity-1");

    assert!(collection.contains_entity("entity-1"));
    assert!(!collection.contains_entity("entity-2"));
}

#[test]
fn test_entity_collection_remove() {
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("a"));
    collection.add(Entity::new("b"));
    collection.add(Entity::new("c"));
    assert_eq!(collection.len(), 3);

    let removed = collection.remove("b");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().id, "b");
    assert_eq!(collection.len(), 2);
    assert!(!collection.contains_entity("b"));
    assert!(collection.contains_entity("a"));
    assert!(collection.contains_entity("c"));
}

#[test]
fn test_entity_collection_remove_all() {
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("x"));
    collection.add(Entity::new("y"));
    collection.remove_all();
    assert_eq!(collection.len(), 0);
    assert!(collection.is_empty());
}

#[test]
fn test_entity_collection_suspend_resume() {
    let mut collection = EntityCollection::new();
    collection.suspend_events();
    collection.add(Entity::new("a"));
    collection.add(Entity::new("b"));
    collection.resume_events();
    // No panic = events were batched correctly
    assert_eq!(collection.len(), 2);
}

#[test]
fn test_data_source_collection_add_remove() {
    let mut dsc = DataSourceCollection::new();
    assert_eq!(dsc.length(), 0);
    assert!(dsc.is_empty());

    let idx = dsc.add("CZML");
    assert_eq!(idx, 0);
    assert_eq!(dsc.length(), 1);

    dsc.add("GeoJSON");
    assert_eq!(dsc.length(), 2);

    let entry = dsc.get(0);
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().name, "CZML");

    let removed = dsc.remove(0);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().name, "CZML");
    assert_eq!(dsc.length(), 1);
}

#[test]
fn test_data_source_collection_move() {
    let mut dsc = DataSourceCollection::new();
    dsc.add("A");
    dsc.add("B");
    dsc.add("C");

    dsc.move_entry(0, 2);
    // [A,B,C] → remove(0)=[B,C] → insert(2,A)=[B,C,A]
    assert_eq!(dsc.get(0).unwrap().name, "B");
    assert_eq!(dsc.get(1).unwrap().name, "C");
    assert_eq!(dsc.get(2).unwrap().name, "A");
}

#[test]
fn test_data_source_collection_index_of() {
    let mut dsc = DataSourceCollection::new();
    dsc.add("Alpha");
    dsc.add("Beta");
    assert_eq!(dsc.index_of("Beta"), Some(1));
    assert_eq!(dsc.index_of("Gamma"), None);
}

#[test]
fn test_custom_data_source_implements_trait() {
    let mut ds = CustomDataSource::new("Test");
    assert_eq!(DataSource::name(&ds), "Test");
    assert!(!ds.is_loading());
    assert!(!ds.is_destroyed());
    assert!(ds.show());

    ds.set_show(false);
    assert!(!ds.show());

    ds.destroy();
    assert!(ds.is_destroyed());
}

#[test]
fn test_custom_data_source_entities() {
    let mut ds = CustomDataSource::new("Test");
    assert_eq!(ds.entities().len(), 0);

    ds.entities_mut().add(Entity::new("e1"));
    assert_eq!(ds.entities().len(), 1);
    assert!(ds.entities().contains_entity("e1"));
}

#[test]
fn test_composite_entity_collection() {
    let mut composite = CompositeEntityCollection::new();
    assert!(composite.is_empty());

    let mut col1 = EntityCollection::new();
    col1.add(Entity::new("a"));
    col1.add(Entity::new("b"));

    let mut col2 = EntityCollection::new();
    col2.add(Entity::new("c"));

    composite.add_collection(col1);
    composite.add_collection(col2);
    assert_eq!(composite.len(), 2);
    assert_eq!(composite.entity_count(), 3);

    assert!(composite.contains("a"));
    assert!(composite.contains("c"));
    assert!(!composite.contains("z"));

    let entity = composite.get_entity("b");
    assert!(entity.is_some());
    assert_eq!(entity.unwrap().id, "b");
}

#[test]
fn test_composite_entity_collection_priority() {
    // Most recently added collection takes priority
    let mut composite = CompositeEntityCollection::new();

    let mut col1 = EntityCollection::new();
    let mut e1 = Entity::new("shared");
    e1.name = Some("First".to_string());
    col1.add(e1);

    let mut col2 = EntityCollection::new();
    let mut e2 = Entity::new("shared");
    e2.name = Some("Second".to_string());
    col2.add(e2);

    composite.add_collection(col1);
    composite.add_collection(col2); // Added later, inserted at index 0

    // col2 was added last, so it's at index 0 and has priority
    let entity = composite.get_entity("shared").unwrap();
    assert_eq!(entity.name.as_deref(), Some("Second"));
}

#[test]
fn test_data_source_display_creation() {
    let dsc = DataSourceCollection::new();
    let display = DataSourceDisplay::new(dsc);
    assert!(!display.is_destroyed());
    assert!(!display.ready());
    assert_eq!(display.data_sources().length(), 0);
}

#[test]
fn test_data_source_display_default_data_source() {
    let dsc = DataSourceCollection::new();
    let mut display = DataSourceDisplay::new(dsc);

    let default_ds = display.default_data_source();
    assert_eq!(default_ds.name, "Default");

    // Add entity to default data source
    display.default_data_source_mut().entities_mut().add(Entity::new("manual"));
    assert_eq!(display.default_data_source().entities().len(), 1);
}

#[test]
fn test_data_source_display_update() {
    let dsc = DataSourceCollection::new();
    let mut display = DataSourceDisplay::new(dsc);

    // Update with no visualizers should return true (vacuously ready)
    let result = display.update(0.0);
    assert!(result);
    assert!(display.ready());
}

#[test]
fn test_data_source_display_destroy() {
    let dsc = DataSourceCollection::new();
    let mut display = DataSourceDisplay::new(dsc);

    assert!(!display.is_destroyed());
    display.destroy();
    assert!(display.is_destroyed());

    // Update after destroy returns false
    assert!(!display.update(0.0));
}

#[test]
fn test_data_source_display_bounding_sphere_not_ready() {
    let dsc = DataSourceCollection::new();
    let display = DataSourceDisplay::new(dsc);

    let entity = Entity::new("test");
    let mut result = [0.0f64; 4];
    let state = display.get_bounding_sphere(&entity, false, &mut result);
    assert_eq!(state, cesium_data_sources::bounding_sphere_state::BoundingSphereState::Pending);
}
