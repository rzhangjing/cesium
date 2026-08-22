//! Mirrors packages/engine/Specs/Core/EventSpec.js
//!
//! DEVIATION: JS listeners are identified by function identity + scope; the
//! Rust port keys them by `ListenerId`. Scope-based `it` blocks are ignored;
//! reentrancy (add/remove while raising) is fully ported. See
//! docs/deviations.md.

use std::cell::Cell;
use std::rc::Rc;

use cesium_core::event::{Event, ListenerId};

// describe("Core/Event")

#[test]
fn works_with_no_scope() {
    let event = Event::<i32>::new();

    let calls = Rc::new(Cell::new(0u32));
    let last_value = Rc::new(Cell::new(0i32));
    let (calls_c, value_c) = (calls.clone(), last_value.clone());

    let callback = event.add_listener(move |value: &i32| {
        calls_c.set(calls_c.get() + 1);
        value_c.set(*value);
    });

    let some_value = 123;
    event.raise_event(&some_value);
    assert_eq!(calls.get(), 1);
    assert_eq!(last_value.get(), some_value);

    calls.set(0);

    event.remove_listener(callback.id());
    event.raise_event(&some_value);
    assert_eq!(calls.get(), 0);
}

#[test]
#[ignore = "listener scope parameter has no Rust counterpart; listeners are keyed by ListenerId (DEVIATION)"]
fn works_with_scope() {}

#[test]
fn can_remove_from_within_a_callback() {
    let event = Rc::new(Event::<()>::new());

    let cb_do_nothing = event.add_listener(|_| {});

    // removeEventCb removes itself while the event is being raised.
    let ev = event.clone();
    let self_id = Rc::new(Cell::new(None::<ListenerId>));
    let self_id_c = self_id.clone();
    let cb_remove_self = event.add_listener(move |_| {
        if let Some(id) = self_id_c.get() {
            ev.remove_listener(id);
        }
    });
    self_id.set(Some(cb_remove_self.id()));

    let cb_do_nothing2 = event.add_listener(|_| {});

    event.raise_event(&());
    assert_eq!(event.number_of_listeners(), 2);

    event.remove_listener(cb_do_nothing.id());
    event.remove_listener(cb_do_nothing2.id());
    assert_eq!(event.number_of_listeners(), 0);
}

/// Helper: adds a listener that removes itself when raised.
fn add_self_removing_listener(event: &Rc<Event<()>>) {
    let ev = event.clone();
    let self_id = Rc::new(Cell::new(None::<ListenerId>));
    let self_id_c = self_id.clone();
    let callback = event.add_listener(move |_| {
        if let Some(id) = self_id_c.get() {
            ev.remove_listener(id);
        }
    });
    self_id.set(Some(callback.id()));
}

#[test]
fn can_remove_multiple_listeners_within_a_callback() {
    let event = Rc::new(Event::<()>::new());

    add_self_removing_listener(&event); // removeEvent0
    event.add_listener(|_| {});
    add_self_removing_listener(&event); // removeEvent2
    event.add_listener(|_| {});
    add_self_removing_listener(&event); // removeEvent4
    event.add_listener(|_| {});
    add_self_removing_listener(&event); // removeEvent6
    event.add_listener(|_| {});
    add_self_removing_listener(&event); // removeEvent8
    event.add_listener(|_| {});

    assert_eq!(event.number_of_listeners(), 10);
    event.raise_event(&());
    assert_eq!(event.number_of_listeners(), 5);
    event.raise_event(&());
    assert_eq!(event.number_of_listeners(), 5);
}

#[test]
fn can_add_a_listener_from_within_a_callback() {
    let event = Rc::new(Event::<()>::new());

    let added_id = Rc::new(Cell::new(None::<ListenerId>));
    let added_id_c = added_id.clone();
    let ev = event.clone();
    let add_event_cb = event.add_listener(move |_| {
        let callback = ev.add_listener(|_| {}); // doNothing
        added_id_c.set(Some(callback.id()));
    });

    event.raise_event(&());
    assert_eq!(event.number_of_listeners(), 2);

    event.remove_listener(added_id.get().expect("doNothing was added"));
    event.remove_listener(add_event_cb.id());
    assert_eq!(event.number_of_listeners(), 0);
}

#[test]
fn can_add_multiple_listeners_within_a_callback() {
    let event = Rc::new(Event::<()>::new());

    let ev = event.clone();
    let add_event0 = event.add_listener(move |_| {
        ev.add_listener(|_| {});
    });
    let ev = event.clone();
    let add_event1 = event.add_listener(move |_| {
        ev.add_listener(|_| {});
    });
    let _ = (add_event0, add_event1);

    assert_eq!(event.number_of_listeners(), 2);
    event.raise_event(&());
    assert_eq!(event.number_of_listeners(), 4);
}

#[test]
fn add_and_remove_works_with_same_callback_registered_twice() {
    // Adaptation of "addEventListener and removeEventListener works with same
    // function of different scopes": JS registers one prototype function
    // under two scopes; in Rust each registration yields an independent
    // ListenerId, and removing one never affects the other.
    let event = Event::<()>::new();

    let times_called1 = Rc::new(Cell::new(0u32));
    let times_called2 = Rc::new(Cell::new(0u32));
    let (c1, c2) = (times_called1.clone(), times_called2.clone());

    let callback1 = event.add_listener(move |_| {
        c1.set(c1.get() + 1);
    });
    let callback2 = event.add_listener(move |_| {
        c2.set(c2.get() + 1);
    });

    event.raise_event(&());
    assert_eq!(times_called1.get(), 1);
    assert_eq!(times_called2.get(), 1);

    event.remove_listener(callback1.id());
    assert_eq!(event.number_of_listeners(), 1);
    event.raise_event(&());

    assert_eq!(times_called1.get(), 1);
    assert_eq!(times_called2.get(), 2);

    event.remove_listener(callback2.id());
    assert_eq!(event.number_of_listeners(), 0);
}

#[test]
fn number_of_listeners_returns_the_correct_number() {
    let event = Event::<()>::new();

    assert_eq!(event.number_of_listeners(), 0);

    let callback1 = event.add_listener(|_| {});
    assert_eq!(event.number_of_listeners(), 1);

    let callback2 = event.add_listener(|_| {});
    assert_eq!(event.number_of_listeners(), 2);

    event.remove_listener(callback2.id());
    assert_eq!(event.number_of_listeners(), 1);
    let _ = callback1;
}

#[test]
fn remove_listener_indicates_if_the_listener_is_registered_with_the_event() {
    let event = Event::<()>::new();

    let callback = event.add_listener(|_| {});
    assert_eq!(event.number_of_listeners(), 1);

    assert!(event.remove_listener(callback.id()));
    assert_eq!(event.number_of_listeners(), 0);

    assert!(!event.remove_listener(callback.id()));
}

#[test]
#[ignore = "listener scope parameter has no Rust counterpart; listeners are keyed by ListenerId (DEVIATION)"]
fn remove_listener_does_not_remove_a_registered_listener_of_a_different_scope() {}

#[test]
fn works_with_no_listeners() {
    let event = Event::<i32>::new();
    event.raise_event(&123);
}

#[test]
fn add_listener_returns_a_function_allowing_removal() {
    let event = Event::<i32>::new();

    let calls = Rc::new(Cell::new(0u32));
    let calls_c = calls.clone();
    let remove = event.add_listener(move |_| {
        calls_c.set(calls_c.get() + 1);
    });

    let some_value = 123;
    event.raise_event(&some_value);
    assert_eq!(calls.get(), 1);

    calls.set(0);

    remove.call(&event);
    event.raise_event(&some_value);
    assert_eq!(calls.get(), 0);
}

#[test]
#[ignore = "listener scope parameter has no Rust counterpart; listeners are keyed by ListenerId (DEVIATION)"]
fn add_listener_with_scope_returns_a_function_allowing_removal() {}

// JS: "addEventListener throws with undefined/null/non-function listener" and
// "removeEventListener throws with undefined/null listener" — the Rust
// signatures (`impl FnMut(&A)` / `ListenerId`) make these cases statically
// impossible. Mirrored as ignored tests for the one-to-one record.

#[test]
#[ignore = "the Rust listener parameter is required by the type system (static Check.typeOf.func)"]
fn add_listener_throws_with_undefined_listener() {}

#[test]
#[ignore = "the Rust listener parameter is required by the type system (static Check.typeOf.func)"]
fn add_listener_throws_with_null_listener() {}

#[test]
#[ignore = "the Rust listener parameter is required by the type system (static Check.typeOf.func)"]
fn add_listener_throws_with_non_function_listener() {}

#[test]
#[ignore = "remove_listener takes a ListenerId; undefined listeners are statically impossible"]
fn remove_listener_throws_with_undefined_listener() {}

#[test]
#[ignore = "remove_listener takes a ListenerId; null listeners are statically impossible"]
fn remove_listener_throws_with_null_listener() {}
