//! Faithful port of CesiumJS DataSources/EntityCollectionSpec.js A-class tests.
//!
//! Original: 29 it() tests. A-class (pure logic, no events/spy/DOM): 15 tests.
//! Event-based tests (collectionChanged, suspendEvents/resumeEvents) are B-class
//! (require event system not yet implemented in Rust).

use cesium_datasource::entity::{Entity, PointGraphics};
use cesium_datasource::entity_collection::EntityCollection;
use cesium_datasource::property::{Color, Property};

// ===========================================================================
// Constructor
// ===========================================================================

#[test]
fn entity_collection_constructor_has_expected_defaults() {
    // "constructor has expected defaults"
    let collection = EntityCollection::new();
    assert_eq!(collection.len(), 0);
    assert!(collection.is_empty());
    assert!(collection.show());
}

// ===========================================================================
// Add / Remove
// ===========================================================================

#[test]
fn entity_collection_add_remove_works() {
    // "add/remove works"
    let mut collection = EntityCollection::new();

    collection.add(Entity::new("e1".to_string()));
    assert_eq!(collection.len(), 1);

    collection.add(Entity::new("e2".to_string()));
    assert_eq!(collection.len(), 2);

    let removed = collection.remove("e2");
    assert!(removed.is_some());
    assert_eq!(collection.len(), 1);

    let removed = collection.remove("e1");
    assert!(removed.is_some());
    assert_eq!(collection.len(), 0);
}

#[test]
fn entity_collection_add_with_id() {
    // "add with template" (adapted: Rust uses Entity::new(id))
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("1".to_string()));

    assert_eq!(collection.len(), 1);
    let entity = collection.get("1").unwrap();
    assert_eq!(entity.id, "1");
}

#[test]
fn entity_collection_add_replaces_same_id() {
    // "add throws for Entity with same id" → In Rust, add replaces (no throw).
    // We verify the replacement semantics instead.
    let mut collection = EntityCollection::new();

    let mut e1 = Entity::new("1".to_string());
    e1.name = Some("first".to_string());
    collection.add(e1);

    let mut e2 = Entity::new("1".to_string());
    e2.name = Some("second".to_string());
    collection.add(e2);

    // Still only 1 entity (replaced, not duplicated)
    assert_eq!(collection.len(), 1);
    assert_eq!(collection.get("1").unwrap().name.as_deref(), Some("second"));
}

// ===========================================================================
// RemoveAll
// ===========================================================================

#[test]
fn entity_collection_remove_all_works() {
    // "removeAll works"
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("e1".to_string()));
    collection.add(Entity::new("e2".to_string()));

    collection.remove_all();
    assert_eq!(collection.len(), 0);
    assert!(collection.is_empty());
}

#[test]
fn entity_collection_remove_all_on_empty_is_noop() {
    // "removeAll raises expected events" (partial: no events, just verify no panic)
    let mut collection = EntityCollection::new();
    collection.remove_all();
    assert_eq!(collection.len(), 0);
}

// ===========================================================================
// RemoveById
// ===========================================================================

#[test]
fn entity_collection_remove_by_id_returns_false_if_not_in_collection() {
    // "removeById returns false if id not in collection."
    let mut collection = EntityCollection::new();
    assert!(!collection.remove_by_id("notThere"));
}

#[test]
fn entity_collection_remove_by_id_returns_true_if_present() {
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("x".to_string()));
    assert!(collection.remove_by_id("x"));
    assert!(!collection.contains("x"));
}

// ===========================================================================
// GetById
// ===========================================================================

#[test]
fn entity_collection_get_by_id_works() {
    // "getById works"
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("a".to_string()));
    collection.add(Entity::new("b".to_string()));

    assert_eq!(collection.get("a").unwrap().id, "a");
    assert_eq!(collection.get("b").unwrap().id, "b");
}

#[test]
fn entity_collection_get_by_id_returns_none_for_nonexistent() {
    // "getById returns undefined for non-existent object"
    let collection = EntityCollection::new();
    assert!(collection.get("123").is_none());
}

// ===========================================================================
// GetOrCreateEntity
// ===========================================================================

#[test]
fn entity_collection_get_or_create_creates_new_if_not_exists() {
    // "getOrCreateEntity creates a new object if it does not exist."
    let mut collection = EntityCollection::new();
    assert_eq!(collection.len(), 0);

    let entity = collection.get_or_create("test");
    assert_eq!(entity.id, "test");
    assert_eq!(collection.len(), 1);
}

#[test]
fn entity_collection_get_or_create_does_not_duplicate() {
    // "getOrCreateEntity does not create a new object if it already exists."
    let mut collection = EntityCollection::new();
    assert_eq!(collection.len(), 0);

    {
        let entity = collection.get_or_create("test");
        assert_eq!(entity.id, "test");
    }
    assert_eq!(collection.len(), 1);

    {
        let entity = collection.get_or_create("test");
        assert_eq!(entity.id, "test");
    }
    assert_eq!(collection.len(), 1);
}

// ===========================================================================
// Contains
// ===========================================================================

#[test]
fn entity_collection_contains_returns_true_if_in_collection() {
    // "contains returns true if in collection"
    let mut collection = EntityCollection::new();
    collection.get_or_create("asd");
    assert!(collection.contains("asd"));
}

#[test]
fn entity_collection_contains_returns_false_if_not_in_collection() {
    // "contains returns false if not in collection"
    let collection = EntityCollection::new();
    assert!(!collection.contains("nonexistent"));
}

// ===========================================================================
// Remove returns None for non-existent
// ===========================================================================

#[test]
fn entity_collection_remove_returns_none_for_nonexistent() {
    // "remove returns false with undefined Entity" (adapted for Rust Option)
    let mut collection = EntityCollection::new();
    assert!(collection.remove("nonexistent").is_none());
}

// ===========================================================================
// Values / Insertion Order
// ===========================================================================

#[test]
fn entity_collection_values_preserves_insertion_order() {
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("c".to_string()));
    collection.add(Entity::new("a".to_string()));
    collection.add(Entity::new("b".to_string()));

    let ids: Vec<&str> = collection.values().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["c", "a", "b"]);
}

#[test]
fn entity_collection_values_after_remove_preserves_order() {
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("a".to_string()));
    collection.add(Entity::new("b".to_string()));
    collection.add(Entity::new("c".to_string()));

    collection.remove("b");

    let ids: Vec<&str> = collection.values().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["a", "c"]);
}

// ===========================================================================
// Show / Visibility
// ===========================================================================

#[test]
fn entity_collection_show_defaults_to_true() {
    let collection = EntityCollection::new();
    assert!(collection.show());
}

#[test]
fn entity_collection_set_show_works() {
    let mut collection = EntityCollection::new();
    collection.set_show(false);
    assert!(!collection.show());
}

#[test]
fn entity_collection_visible_entities_filters_by_entity_show() {
    let mut collection = EntityCollection::new();

    let mut visible = Entity::new("v1".to_string());
    visible.show = true;
    visible.point = Some(PointGraphics::default());
    collection.add(visible);

    let mut hidden = Entity::new("h1".to_string());
    hidden.show = false;
    hidden.point = Some(PointGraphics::default());
    collection.add(hidden);

    assert_eq!(collection.visible_entities().count(), 1);
}

#[test]
fn entity_collection_visible_entities_respects_collection_show() {
    let mut collection = EntityCollection::new();

    let mut e = Entity::new("e1".to_string());
    e.show = true;
    e.point = Some(PointGraphics::default());
    collection.add(e);

    assert_eq!(collection.visible_entities().count(), 1);

    collection.set_show(false);
    assert_eq!(collection.visible_entities().count(), 0);
}

// ===========================================================================
// Renderable entities
// ===========================================================================

#[test]
fn entity_collection_renderable_entities_filters_by_graphics() {
    let mut collection = EntityCollection::new();

    // Has graphics
    let mut with_gfx = Entity::new("gfx".to_string());
    with_gfx.point = Some(PointGraphics {
        color: Property::Constant(Color::RED),
        ..Default::default()
    });
    collection.add(with_gfx);

    // No graphics
    collection.add(Entity::new("no-gfx".to_string()));

    assert_eq!(collection.renderable_entities().count(), 1);
}

// ===========================================================================
// IDs accessor
// ===========================================================================

#[test]
fn entity_collection_ids_returns_insertion_order() {
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("z".to_string()));
    collection.add(Entity::new("y".to_string()));
    collection.add(Entity::new("x".to_string()));

    assert_eq!(
        collection.ids(),
        &["z".to_string(), "y".to_string(), "x".to_string()]
    );
}
