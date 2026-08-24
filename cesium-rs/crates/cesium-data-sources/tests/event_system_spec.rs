//! Event-system specs for the DataSources port.
//!
//! Mirrors the event-related cases of:
//! - `packages/engine/Specs/Core/EventSpec.js` (add/remove/raise semantics,
//!   reentrant removal/addition while raising) — the `Event` model itself
//!   lives in `cesium-core`; a compact semantic mirror is included here so
//!   the DataSources event pipeline is covered end to end.
//! - `packages/engine/Specs/DataSources/ConstantPropertySpec.js`
//!   (definitionChanged on setValue)
//! - `packages/engine/Specs/DataSources/EntitySpec.js`
//!   (definitionChanged raised on property changes, addProperty /
//!   removeProperty)
//! - `packages/engine/Specs/DataSources/EntityCollectionSpec.js`
//!   (collectionChanged on add/remove/change, suspendEvents /
//!   resumeEvents batching and recovery)
//! - `packages/engine/Specs/DataSources/PropertyBagSpec.js`
//!   (definitionChanged on add/remove)

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::event::Event;
use cesium_data_sources::callback_property::CallbackProperty;
use cesium_data_sources::constant_position_property::ConstantPositionProperty;
use cesium_data_sources::constant_property::ConstantProperty;
use cesium_data_sources::entity::Entity;
use cesium_data_sources::entity_collection::{CollectionChangedArgs, EntityCollection};
use cesium_data_sources::position_property::PositionReferenceFrame;
use cesium_data_sources::property::{Property, PropertyResult};
use cesium_data_sources::property_bag::PropertyBag;
use cesium_test_utils::expect_to_throw_dev_error;

// ============================================================================
// Core/Event semantics mirror (add/remove/raise, reentrancy)
// ============================================================================

#[test]
fn event_add_remove_raise_and_number_of_listeners() {
    let event = Event::<i32>::new();
    assert_eq!(event.number_of_listeners(), 0);

    let calls = Rc::new(Cell::new(0u32));
    let last = Rc::new(Cell::new(0i32));
    let (calls_c, last_c) = (calls.clone(), last.clone());
    let callback = event.add_listener(move |value: &i32| {
        calls_c.set(calls_c.get() + 1);
        last_c.set(*value);
    });
    assert_eq!(event.number_of_listeners(), 1);

    event.raise_event(&123);
    assert_eq!(calls.get(), 1);
    assert_eq!(last.get(), 123);

    assert!(event.remove_listener(callback.id()));
    assert_eq!(event.number_of_listeners(), 0);

    event.raise_event(&456);
    assert_eq!(calls.get(), 1); // unchanged after removal

    // Removing an unregistered listener returns false (JS parity).
    assert!(!event.remove_listener(callback.id()));
}

#[test]
fn event_can_remove_a_listener_from_within_a_callback() {
    // JS EventSpec: "can remove from within a callback" — the listener is
    // still invoked once, then removed after the raise completes.
    let event = Rc::new(Event::<()>::new());

    let invocations = Rc::new(Cell::new(0u32));

    let ev = event.clone();
    let self_id = Rc::new(Cell::new(None::<cesium_core::event::ListenerId>));
    let self_id_c = self_id.clone();
    let invocations_c = invocations.clone();
    let remove_self = event.add_listener(move |_| {
        invocations_c.set(invocations_c.get() + 1);
        if let Some(id) = self_id_c.get() {
            ev.remove_listener(id);
        }
    });
    self_id.set(Some(remove_self.id()));

    event.raise_event(&());
    assert_eq!(invocations.get(), 1);
    // Removal is deferred until the raise finishes: during the raise the
    // count still reports 1, afterwards 0 (JS `_toRemove` semantics).
    assert_eq!(event.number_of_listeners(), 0);

    event.raise_event(&());
    assert_eq!(invocations.get(), 1); // not invoked again
}

#[test]
fn event_can_add_a_listener_from_within_a_callback() {
    // JS EventSpec: "can add a listener from within a callback" — the new
    // listener is not invoked by the in-progress raise, but is registered
    // afterwards (JS `_toAdd` semantics).
    let event = Rc::new(Event::<()>::new());

    let second_calls = Rc::new(Cell::new(0u32));
    let second_calls_outer = second_calls.clone();
    let ev = event.clone();
    let _first = event.add_listener(move |_| {
        let second_calls_c = second_calls_outer.clone();
        ev.add_listener(move |_| {
            second_calls_c.set(second_calls_c.get() + 1);
        });
    });

    event.raise_event(&());
    assert_eq!(second_calls.get(), 0); // deferred addition, not invoked yet
    assert_eq!(event.number_of_listeners(), 2);

    event.raise_event(&());
    assert_eq!(second_calls.get(), 1);
}

// ============================================================================
// ConstantProperty / value-model definitionChanged
// ============================================================================

#[test]
fn constant_property_set_value_raises_definition_changed() {
    // ConstantPropertySpec: "setValue raises definitionChanged"
    let mut property = ConstantProperty::new(PropertyResult::Number(1.0));

    let raised = Rc::new(Cell::new(0u32));
    let raised_c = raised.clone();
    property
        .definition_changed_event()
        .add_listener(move |_| raised_c.set(raised_c.get() + 1));

    property.set_value(PropertyResult::Number(2.0));
    assert_eq!(raised.get(), 1);
    assert_eq!(property.get_value(0.0), PropertyResult::Number(2.0));
}

#[test]
fn constant_property_set_value_does_not_raise_for_equal_value() {
    // ConstantPropertySpec: setting an equal value does not raise.
    let mut property = ConstantProperty::new(PropertyResult::Number(5.0));

    let raised = Rc::new(Cell::new(0u32));
    let raised_c = raised.clone();
    property
        .definition_changed_event()
        .add_listener(move |_| raised_c.set(raised_c.get() + 1));

    property.set_value(PropertyResult::Number(5.0));
    assert_eq!(raised.get(), 0);
}

#[test]
fn constant_property_exposes_definition_changed_through_property_trait() {
    let property = ConstantProperty::new(PropertyResult::Boolean(true));
    let as_trait: &dyn Property = &property;
    assert!(as_trait.definition_changed().is_some());
}

#[test]
fn constant_position_property_set_value_raises_definition_changed() {
    let mut property = ConstantPositionProperty::new(Cartesian3::new(1.0, 2.0, 3.0));

    let raised = Rc::new(Cell::new(0u32));
    let raised_c = raised.clone();
    property
        .definition_changed_event()
        .add_listener(move |_| raised_c.set(raised_c.get() + 1));

    property.set_value(Cartesian3::new(4.0, 5.0, 6.0));
    assert_eq!(raised.get(), 1);

    // Equal value: no event.
    property.set_value(Cartesian3::new(4.0, 5.0, 6.0));
    assert_eq!(raised.get(), 1);

    // Reference frame change raises; identical frame does not.
    property.set_reference_frame(PositionReferenceFrame::Inertial);
    assert_eq!(raised.get(), 2);
    property.set_reference_frame(PositionReferenceFrame::Inertial);
    assert_eq!(raised.get(), 2);
}

#[test]
fn callback_property_set_callback_raises_definition_changed() {
    let mut property =
        CallbackProperty::new(Box::new(|_| PropertyResult::Number(1.0)), true);

    let raised = Rc::new(Cell::new(0u32));
    let raised_c = raised.clone();
    property
        .definition_changed_event()
        .add_listener(move |_| raised_c.set(raised_c.get() + 1));

    property.set_callback(Box::new(|_| PropertyResult::Number(2.0)), false);
    assert_eq!(raised.get(), 1);
    assert!(!property.is_constant());
    assert_eq!(property.get_value(0.0), PropertyResult::Number(2.0));
}

#[test]
fn property_bag_raises_definition_changed_on_structural_changes() {
    let mut bag = PropertyBag::new();

    let raised = Rc::new(Cell::new(0u32));
    let raised_c = raised.clone();
    bag.definition_changed_event()
        .add_listener(move |_| raised_c.set(raised_c.get() + 1));

    // Adding a new entry raises.
    bag.set("a", PropertyResult::Number(1.0));
    assert_eq!(raised.get(), 1);

    // Replacing with a different value raises.
    bag.set("a", PropertyResult::Number(2.0));
    assert_eq!(raised.get(), 2);

    // Replacing with an equal value does not raise.
    bag.set("a", PropertyResult::Number(2.0));
    assert_eq!(raised.get(), 2);

    // Removing an existing entry raises; removing a missing one does not.
    assert!(bag.remove("a").is_some());
    assert_eq!(raised.get(), 3);
    assert!(bag.remove("a").is_none());
    assert_eq!(raised.get(), 3);

    // clear raises only when the bag is non-empty.
    bag.set("b", PropertyResult::Boolean(true));
    assert_eq!(raised.get(), 4);
    bag.clear();
    assert_eq!(raised.get(), 5);
    bag.clear();
    assert_eq!(raised.get(), 5);
}

// ============================================================================
// Entity definitionChanged
// ============================================================================

#[test]
fn entity_set_name_raises_definition_changed_with_old_and_new_values() {
    // EntitySpec: "definitionChanged is raised when a property changes"
    let mut entity = Entity::new("id");

    let args = Rc::new(RefCell::new(Vec::new()));
    let args_c = args.clone();
    entity.definition_changed.add_listener(move |event_args| {
        args_c.borrow_mut().push(event_args.clone());
    });

    entity.set_name(Some("NewName".to_string()));
    let captured = args.borrow();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].property_name, "name");
    assert_eq!(
        captured[0].new_value,
        PropertyResult::String("NewName".to_string())
    );
    assert_eq!(captured[0].old_value, PropertyResult::None);
}

#[test]
fn entity_setters_do_not_raise_when_the_value_is_unchanged() {
    let mut entity = Entity::new("id");
    entity.set_name(Some("name".to_string()));

    let raised = Rc::new(Cell::new(0u32));
    let raised_c = raised.clone();
    entity.definition_changed.add_listener(move |_| {
        raised_c.set(raised_c.get() + 1);
    });

    entity.set_name(Some("name".to_string()));
    entity.set_show(true); // show defaults to true
    entity.set_position(None);
    entity.set_description(None);
    entity.set_parent_id(None);
    entity.set_view_from(None);
    assert_eq!(raised.get(), 0);
}

#[test]
fn entity_set_show_raises_with_value_and_negated_old_value() {
    // Entity.js: `raiseEvent(this, "show", value, !value)`
    let mut entity = Entity::new("id");

    let args = Rc::new(RefCell::new(Vec::new()));
    let args_c = args.clone();
    entity.definition_changed.add_listener(move |event_args| {
        args_c.borrow_mut().push(event_args.clone());
    });

    entity.set_show(false);
    let captured = args.borrow();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].property_name, "show");
    assert_eq!(captured[0].new_value, PropertyResult::Boolean(false));
    assert_eq!(captured[0].old_value, PropertyResult::Boolean(true));
}

#[test]
fn entity_set_position_raises_with_cartesian_payload() {
    let mut entity = Entity::new("id");

    let args = Rc::new(RefCell::new(Vec::new()));
    let args_c = args.clone();
    entity.definition_changed.add_listener(move |event_args| {
        args_c.borrow_mut().push(event_args.clone());
    });

    entity.set_position(Some(Cartesian3::new(1.0, 2.0, 3.0)));
    entity.set_position(None);

    let captured = args.borrow();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0].property_name, "position");
    assert_eq!(
        captured[0].new_value,
        PropertyResult::Cartesian3(1.0, 2.0, 3.0)
    );
    assert_eq!(captured[0].old_value, PropertyResult::None);
    assert_eq!(captured[1].new_value, PropertyResult::None);
    assert_eq!(
        captured[1].old_value,
        PropertyResult::Cartesian3(1.0, 2.0, 3.0)
    );
}

#[test]
fn entity_set_parent_id_raises_definition_changed() {
    let mut entity = Entity::new("child");

    let args = Rc::new(RefCell::new(Vec::new()));
    let args_c = args.clone();
    entity.definition_changed.add_listener(move |event_args| {
        args_c.borrow_mut().push(event_args.clone());
    });

    entity.set_parent_id(Some("parent".to_string()));
    let captured = args.borrow();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].property_name, "parent");
    assert_eq!(
        captured[0].new_value,
        PropertyResult::String("parent".to_string())
    );
}

#[test]
fn entity_merge_raises_definition_changed_for_effective_changes() {
    let mut target = Entity::new("target");
    let mut source = Entity::new("source");
    source.name = Some("merged".to_string());
    source.position = Some(Cartesian3::new(7.0, 8.0, 9.0));

    let raised = Rc::new(Cell::new(0u32));
    let raised_c = raised.clone();
    target.definition_changed.add_listener(move |_| {
        raised_c.set(raised_c.get() + 1);
    });

    target.merge(&source);
    // name + position changed → 2 events.
    assert_eq!(raised.get(), 2);

    // Merging the same source again changes nothing.
    target.merge(&source);
    assert_eq!(raised.get(), 2);
}

#[test]
fn entity_add_and_remove_property_manage_property_names() {
    // EntitySpec: "addProperty adds a property" / "removeProperty removes"
    let mut entity = Entity::new("id");
    assert!(!entity
        .property_names()
        .iter()
        .any(|name| name == "custom"));

    entity.add_property("custom");
    assert!(entity
        .property_names()
        .iter()
        .any(|name| name == "custom"));

    entity.remove_property("custom");
    assert!(!entity
        .property_names()
        .iter()
        .any(|name| name == "custom"));
}

#[test]
fn entity_add_property_throws_for_duplicate_and_reserved_names() {
    // EntitySpec: addProperty DeveloperError cases (debug-gated in JS).
    let mut entity = Entity::new("id");
    entity.add_property("custom");

    let message = expect_to_throw_dev_error(|| entity.add_property("custom"));
    assert!(message.contains("custom is already a registered property."));

    let message = expect_to_throw_dev_error(|| entity.add_property("id"));
    assert!(message.contains("id is a reserved property name."));
}

#[test]
fn entity_remove_property_throws_for_unregistered_name() {
    let mut entity = Entity::new("id");
    let message = expect_to_throw_dev_error(|| entity.remove_property("missing"));
    assert!(message.contains("missing is not a registered property."));
}

#[test]
fn entity_custom_property_setter_raises_definition_changed() {
    let mut entity = Entity::new("id");
    entity.add_property("custom");

    let args = Rc::new(RefCell::new(Vec::new()));
    let args_c = args.clone();
    entity.definition_changed.add_listener(move |event_args| {
        args_c.borrow_mut().push(event_args.clone());
    });

    entity.set_custom_property("custom", PropertyResult::Number(42.0));
    // Setting the same value again does not raise.
    entity.set_custom_property("custom", PropertyResult::Number(42.0));

    let captured = args.borrow();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].property_name, "custom");
    assert_eq!(captured[0].new_value, PropertyResult::Number(42.0));
    assert_eq!(captured[0].old_value, PropertyResult::None);
    assert_eq!(
        entity.get_custom_property("custom"),
        Some(&PropertyResult::Number(42.0))
    );
}

// ============================================================================
// EntityCollection collectionChanged + suspend/resume
// ============================================================================

fn record_collection_changed(
    collection: &EntityCollection,
) -> Rc<RefCell<Vec<CollectionChangedArgs>>> {
    let events = Rc::new(RefCell::new(Vec::new()));
    let events_c = events.clone();
    collection
        .collection_changed()
        .add_listener(move |args| events_c.borrow_mut().push(args.clone()));
    events
}

#[test]
fn collection_changed_is_raised_when_an_entity_is_added() {
    // EntityCollectionSpec: "collectionChanged is raised when an entity is
    // added"
    let mut collection = EntityCollection::new();
    let events = record_collection_changed(&collection);

    collection.add(Entity::new("a"));

    let captured = events.borrow();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].added, vec!["a".to_string()]);
    assert!(captured[0].removed.is_empty());
    assert!(captured[0].changed.is_empty());
}

#[test]
fn collection_changed_is_raised_when_an_entity_is_removed() {
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("a"));

    let events = record_collection_changed(&collection);
    assert!(collection.remove("a").is_some());

    let captured = events.borrow();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].removed, vec!["a".to_string()]);
    assert!(captured[0].added.is_empty());
}

#[test]
fn collection_changed_is_raised_when_remove_all_is_called() {
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("a"));
    collection.add(Entity::new("b"));

    let events = record_collection_changed(&collection);
    collection.remove_all();

    let captured = events.borrow();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].removed.len(), 2);
    assert!(captured[0].removed.contains(&"a".to_string()));
    assert!(captured[0].removed.contains(&"b".to_string()));
}

#[test]
fn entity_definition_change_bubbles_to_collection_changed() {
    // EntityCollectionSpec: "collectionChanged is raised when an entity
    // changes" (JS wires this through the per-entity definitionChanged
    // subscription installed on add).
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("a"));

    let events = record_collection_changed(&collection);

    collection
        .get_by_id_mut("a")
        .unwrap()
        .set_name(Some("renamed".to_string()));

    let captured = events.borrow();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].changed, vec!["a".to_string()]);
    assert!(captured[0].added.is_empty());
    assert!(captured[0].removed.is_empty());
}

#[test]
fn removed_entity_no_longer_notifies_the_collection() {
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("a"));
    let entity = collection.remove("a").unwrap();

    let events = record_collection_changed(&collection);

    // The collection unsubscribed on removal: mutating the entity must not
    // fire collectionChanged.
    let mut entity = entity;
    entity.set_name(Some("after-removal".to_string()));
    assert!(events.borrow().is_empty());
}

#[test]
fn suspend_events_prevents_events_until_resume() {
    // EntityCollectionSpec: "suspendEvents prevents events from being
    // raised until resumeEvents is called" — all suspended operations are
    // covered by a single event.
    let mut collection = EntityCollection::new();
    let events = record_collection_changed(&collection);

    collection.suspend_events();
    collection.add(Entity::new("a"));
    collection.add(Entity::new("b"));
    collection.remove("a");
    assert!(events.borrow().is_empty());

    collection.resume_events();

    let captured = events.borrow();
    assert_eq!(captured.len(), 1);
    // "b" was added during suspension; "a" was added then removed, so it
    // cancels out of both lists (JS addedEntities/removedEntities interplay).
    assert_eq!(captured[0].added, vec!["b".to_string()]);
    assert!(captured[0].removed.is_empty());
}

#[test]
fn suspended_entity_changes_are_recovered_on_resume() {
    // Regression for the audit finding: changes accumulated while events
    // were suspended must be dispatched on resume.
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("a"));

    let events = record_collection_changed(&collection);

    collection.suspend_events();
    collection
        .get_by_id_mut("a")
        .unwrap()
        .set_show(false);
    collection
        .get_by_id_mut("a")
        .unwrap()
        .set_position(Some(Cartesian3::new(1.0, 2.0, 3.0)));
    assert!(events.borrow().is_empty());

    collection.resume_events();

    let captured = events.borrow();
    assert_eq!(captured.len(), 1);
    // Both changes collapse into a single "changed" entry for the entity.
    assert_eq!(captured[0].changed, vec!["a".to_string()]);
}

#[test]
fn entity_changed_while_pending_add_is_not_reported_as_changed() {
    // JS `_onEntityDefinitionChanged`: entities still listed in
    // `_addedEntities` are not duplicated into `_changedEntities`.
    let mut collection = EntityCollection::new();
    let events = record_collection_changed(&collection);

    collection.suspend_events();
    collection.add(Entity::new("a"));
    collection
        .get_by_id_mut("a")
        .unwrap()
        .set_name(Some("renamed".to_string()));
    collection.resume_events();

    let captured = events.borrow();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].added, vec!["a".to_string()]);
    assert!(captured[0].changed.is_empty());
}

#[test]
fn suspend_events_is_reference_counted() {
    // EntityCollectionSpec: suspendEvents/resumeEvents can be nested.
    let mut collection = EntityCollection::new();
    let events = record_collection_changed(&collection);

    collection.suspend_events();
    collection.suspend_events();
    collection.add(Entity::new("a"));
    collection.resume_events();
    // Still suspended one level deep: no event yet.
    assert!(events.borrow().is_empty());

    collection.resume_events();
    assert_eq!(events.borrow().len(), 1);
}

#[test]
fn resume_events_throws_when_not_suspended() {
    // EntityCollectionSpec: "resumeEvents throws if called before
    // suspendEvents" (debug-gated DeveloperError in CesiumJS).
    let mut collection = EntityCollection::new();
    let message = expect_to_throw_dev_error(|| collection.resume_events());
    assert!(message.contains("resumeEvents can not be called before suspendEvents."));
}

#[test]
fn add_throws_when_the_entity_id_already_exists() {
    // EntityCollectionSpec: "add throws if the entity id already exists"
    // (not debug-gated in CesiumJS).
    let mut collection = EntityCollection::new();
    collection.add(Entity::new("a"));
    let message = expect_to_throw_dev_error(|| {
        collection.add(Entity::new("a"));
    });
    assert!(message.contains("An entity with id a already exists"));
}

#[test]
fn get_or_create_entity_reuses_existing_and_adds_new() {
    let mut collection = EntityCollection::new();
    let events = record_collection_changed(&collection);

    {
        let entity = collection.get_or_create_entity("a");
        entity.set_name(Some("created".to_string()));
    }
    {
        let entity = collection.get_or_create_entity("a");
        assert_eq!(entity.name.as_deref(), Some("created"));
    }

    assert_eq!(collection.length(), 1);
    let captured = events.borrow();
    // One add event + one change event (name set after add).
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0].added, vec!["a".to_string()]);
    assert_eq!(captured[1].changed, vec!["a".to_string()]);
}
