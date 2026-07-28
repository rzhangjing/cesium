//! Event system specs
//! Ported from CesiumJS Core/Event.js
//!
//! A-class tests: add/remove/raise/clear/number_of_listeners/multiple listeners/
//! SimpleEvent/typed args

use cesium_event::{Event, SimpleEvent};
use std::cell::Cell;
use std::rc::Rc;

// ─── Basic Event ───────────────────────────────────────────────────────────────

#[test]
fn event_new_is_empty() {
    let event: Event<i32> = Event::new();
    assert_eq!(event.number_of_listeners(), 0);
    assert!(event.is_empty());
}

#[test]
fn event_add_listener_increments_count() {
    let event: Event<()> = Event::new();
    let id1 = event.add_listener(|_| {});
    assert_eq!(event.number_of_listeners(), 1);
    assert!(!event.is_empty());

    let _id2 = event.add_listener(|_| {});
    assert_eq!(event.number_of_listeners(), 2);

    // ListenerId is unique
    let _ = id1;
}

#[test]
fn event_raise_calls_listener() {
    let event: Event<i32> = Event::new();
    let received = Rc::new(Cell::new(0));
    let received_clone = received.clone();

    event.add_listener(move |val| {
        received_clone.set(*val);
    });

    event.raise(&42);
    assert_eq!(received.get(), 42);

    event.raise(&100);
    assert_eq!(received.get(), 100);
}

#[test]
fn event_multiple_listeners_all_called() {
    let event: Event<i32> = Event::new();
    let sum = Rc::new(Cell::new(0));

    let sum1 = sum.clone();
    event.add_listener(move |val| {
        sum1.set(sum1.get() + val);
    });

    let sum2 = sum.clone();
    event.add_listener(move |val| {
        sum2.set(sum2.get() + val * 2);
    });

    let sum3 = sum.clone();
    event.add_listener(move |val| {
        sum3.set(sum3.get() + val * 3);
    });

    event.raise(&10);
    assert_eq!(sum.get(), 60); // 10 + 20 + 30
}

#[test]
fn event_remove_listener() {
    let event: Event<i32> = Event::new();
    let count = Rc::new(Cell::new(0));
    let count_clone = count.clone();

    let id = event.add_listener(move |_| {
        count_clone.set(count_clone.get() + 1);
    });

    event.raise(&0);
    assert_eq!(count.get(), 1);

    assert!(event.remove_listener(id));
    assert_eq!(event.number_of_listeners(), 0);

    event.raise(&0);
    assert_eq!(count.get(), 1); // Not called after removal
}

#[test]
fn event_remove_nonexistent_returns_false() {
    let event: Event<()> = Event::new();
    let id = event.add_listener(|_| {});
    assert!(event.remove_listener(id));
    // Second removal fails
    assert!(!event.remove_listener(id));
}

#[test]
fn event_clear_removes_all() {
    let event: Event<()> = Event::new();
    event.add_listener(|_| {});
    event.add_listener(|_| {});
    event.add_listener(|_| {});
    assert_eq!(event.number_of_listeners(), 3);

    event.clear();
    assert_eq!(event.number_of_listeners(), 0);
    assert!(event.is_empty());
}

#[test]
fn event_raise_with_no_listeners_is_noop() {
    let event: Event<String> = Event::new();
    // Should not panic
    event.raise(&"hello".to_string());
}

// ─── SimpleEvent ───────────────────────────────────────────────────────────────

#[test]
fn simple_event_raise_simple() {
    let event = SimpleEvent::new();
    let fired = Rc::new(Cell::new(false));
    let fired_clone = fired.clone();

    event.add_listener(move |_| {
        fired_clone.set(true);
    });

    assert!(!fired.get());
    event.raise_simple();
    assert!(fired.get());
}

// ─── Typed args ────────────────────────────────────────────────────────────────

#[test]
fn event_with_tuple_args() {
    let event: Event<(f64, f64)> = Event::new();
    let result = Rc::new(Cell::new(0.0));
    let result_clone = result.clone();

    event.add_listener(move |(x, y)| {
        result_clone.set(x + y);
    });

    event.raise(&(3.0, 4.0));
    assert!((result.get() - 7.0).abs() < 1e-10);
}

#[test]
fn event_listener_ids_are_unique() {
    let event: Event<()> = Event::new();
    let id1 = event.add_listener(|_| {});
    let id2 = event.add_listener(|_| {});
    let id3 = event.add_listener(|_| {});

    // All different
    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert_ne!(id1, id3);
}
