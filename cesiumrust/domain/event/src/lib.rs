//! cesium-event: Type-safe event system.
//! Domain layer - pure Rust, no framework dependency.
//!
//! CesiumJS mapping: `packages/engine/Source/Core/Event.js`

use std::cell::RefCell;
use std::collections::HashMap;

/// A unique identifier for an event listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListenerId(u64);

/// Type alias for the listener map to reduce complexity.
type ListenerMap<Args> = HashMap<u64, Box<dyn Fn(&Args)>>;

/// A generic event that can have multiple listeners.
/// Maps to CesiumJS `Event`
///
/// Type parameter `Args` is a tuple of argument types passed to listeners.
pub struct Event<Args: Clone> {
    listeners: RefCell<ListenerMap<Args>>,
    next_id: RefCell<u64>,
}

impl<Args: Clone> Event<Args> {
    /// Creates a new empty event.
    pub fn new() -> Self {
        Self {
            listeners: RefCell::new(HashMap::new()),
            next_id: RefCell::new(0),
        }
    }

    /// Returns the number of listeners currently subscribed.
    /// Maps to `Event.numberOfListeners`
    pub fn number_of_listeners(&self) -> usize {
        self.listeners.borrow().len()
    }

    /// Returns true if there are no listeners.
    pub fn is_empty(&self) -> bool {
        self.listeners.borrow().is_empty()
    }

    /// Registers a callback function to be executed whenever the event is raised.
    /// Maps to `Event.addEventListener`
    ///
    /// Returns a `ListenerId` that can be used to remove the listener.
    pub fn add_listener<F>(&self, listener: F) -> ListenerId
    where
        F: Fn(&Args) + 'static,
    {
        let mut next_id = self.next_id.borrow_mut();
        let id = *next_id;
        *next_id += 1;

        self.listeners.borrow_mut().insert(id, Box::new(listener));
        ListenerId(id)
    }

    /// Unregisters a previously registered callback.
    /// Maps to `Event.removeEventListener`
    ///
    /// Returns true if the listener was removed.
    pub fn remove_listener(&self, id: ListenerId) -> bool {
        self.listeners.borrow_mut().remove(&id.0).is_some()
    }

    /// Raises the event by calling each registered listener with the given arguments.
    /// Maps to `Event.raiseEvent`
    pub fn raise(&self, args: &Args) {
        let listeners = self.listeners.borrow();
        for listener in listeners.values() {
            listener(args);
        }
    }

    /// Removes all listeners.
    pub fn clear(&self) {
        self.listeners.borrow_mut().clear();
    }
}

impl<Args: Clone> Default for Event<Args> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Args: Clone> std::fmt::Debug for Event<Args> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Event")
            .field("listener_count", &self.number_of_listeners())
            .finish()
    }
}

/// A simple event with no arguments.
pub type SimpleEvent = Event<()>;

impl SimpleEvent {
    /// Raises the event with no arguments.
    pub fn raise_simple(&self) {
        self.raise(&());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn test_add_and_raise() {
        let event: Event<i32> = Event::new();
        let received = Rc::new(Cell::new(0));
        let received_clone = received.clone();

        event.add_listener(move |val| {
            received_clone.set(*val);
        });

        event.raise(&42);
        assert_eq!(received.get(), 42);
    }

    #[test]
    fn test_multiple_listeners() {
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

        event.raise(&10);
        assert_eq!(sum.get(), 30); // 10 + 20
    }

    #[test]
    fn test_remove_listener() {
        let event: Event<i32> = Event::new();
        let count = Rc::new(Cell::new(0));
        let count_clone = count.clone();

        let id = event.add_listener(move |_| {
            count_clone.set(count_clone.get() + 1);
        });

        event.raise(&0);
        assert_eq!(count.get(), 1);

        assert!(event.remove_listener(id));
        event.raise(&0);
        assert_eq!(count.get(), 1); // Should not increment
    }

    #[test]
    fn test_number_of_listeners() {
        let event: Event<()> = Event::new();
        assert_eq!(event.number_of_listeners(), 0);

        let id1 = event.add_listener(|_| {});
        assert_eq!(event.number_of_listeners(), 1);

        let _id2 = event.add_listener(|_| {});
        assert_eq!(event.number_of_listeners(), 2);

        event.remove_listener(id1);
        assert_eq!(event.number_of_listeners(), 1);
    }

    #[test]
    fn test_simple_event() {
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

    #[test]
    fn test_clear() {
        let event: Event<()> = Event::new();
        event.add_listener(|_| {});
        event.add_listener(|_| {});
        assert_eq!(event.number_of_listeners(), 2);

        event.clear();
        assert_eq!(event.number_of_listeners(), 0);
    }
}
