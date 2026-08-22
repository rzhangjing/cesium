//! Ported from packages/engine/Source/Core/Event.js
//!
//! DEVIATION: JS listeners are keyed by function identity + scope; the Rust
//! port keys listeners by an opaque [`ListenerId`] token returned from
//! [`Event::add_listener`]. Interior mutability (`RefCell`) is used so that
//! listeners may add/remove listeners while the event is being raised,
//! mirroring CesiumJS reentrancy semantics. See docs/deviations.md.

use std::cell::{Cell, RefCell};

use crate::check::type_of;

/// Identifies a registered listener (Rust stand-in for JS function
/// identity + scope pair).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ListenerId(u64);

/// A function that removes a listener.
///
/// Port of the `Event.RemoveCallback` callback. JS captures the event in the
/// closure; the Rust variant takes the event explicitly on [`call`].
///
/// [`call`]: RemoveCallback::call
pub struct RemoveCallback<A = ()> {
    id: ListenerId,
    _marker: std::marker::PhantomData<fn(A)>,
}

impl<A> RemoveCallback<A> {
    /// The listener id targeted by this callback.
    #[must_use]
    pub fn id(&self) -> ListenerId {
        self.id
    }

    /// Invokes the callback, removing the associated listener from `event`.
    pub fn call(self, event: &Event<A>) -> bool {
        event.remove_listener(self.id)
    }
}

struct ListenerEntry<A> {
    id: ListenerId,
    /// `None` only while the listener is currently being invoked (the box is
    /// taken out so that listeners may re-enter `add_listener` /
    /// `remove_listener`, which borrow the list).
    listener: Option<Box<dyn FnMut(&A)>>,
}

/// A generic utility class for managing subscribers for a particular event.
/// This class is usually instantiated inside of a container class and
/// exposed as a property for others to subscribe to.
///
/// `A` is the argument payload passed to listeners on [`Event::raise_event`]
/// (`()` for events with no arguments).
pub struct Event<A = ()> {
    listeners: RefCell<Vec<ListenerEntry<A>>>,
    to_add: RefCell<Vec<ListenerEntry<A>>>,
    to_remove: RefCell<Vec<ListenerId>>,
    invoking_listeners: Cell<bool>,
    listener_count: Cell<usize>, // Tracks number of listener + scope pairs
    next_id: Cell<u64>,
}

impl<A> Default for Event<A> {
    fn default() -> Self {
        Self {
            listeners: RefCell::new(Vec::new()),
            to_add: RefCell::new(Vec::new()),
            to_remove: RefCell::new(Vec::new()),
            invoking_listeners: Cell::new(false),
            listener_count: Cell::new(0),
            next_id: Cell::new(1),
        }
    }
}

impl<A> Event<A> {
    /// Port of `new Event()`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of listeners currently subscribed to the event.
    ///
    /// Port of the `numberOfListeners` getter.
    #[must_use]
    pub fn number_of_listeners(&self) -> usize {
        self.listener_count.get()
    }

    /// Registers a callback function to be executed whenever the event is
    /// raised.
    ///
    /// Port of `Event.prototype.addEventListener`; returns a
    /// [`RemoveCallback`] that will remove this event listener when invoked.
    pub fn add_listener(&self, listener: impl FnMut(&A) + 'static) -> RemoveCallback<A> {
        // >>includeStart('debug', pragmas.debug) — Check.typeOf.func("listener", listener)
        type_of::func("listener", true);

        let id = ListenerId(self.next_id.get());
        self.next_id.set(self.next_id.get() + 1);
        let entry = ListenerEntry {
            id,
            listener: Some(Box::new(listener)),
        };

        if self.invoking_listeners.get() {
            self.to_add.borrow_mut().push(entry);
        } else {
            self.listeners.borrow_mut().push(entry);
        }
        self.listener_count.set(self.listener_count.get() + 1);

        RemoveCallback {
            id,
            _marker: std::marker::PhantomData,
        }
    }

    /// Unregisters a previously registered callback.
    ///
    /// Port of `Event.prototype.removeEventListener`. Returns `true` if the
    /// listener was removed; `false` if the listener id is not registered
    /// with the event.
    pub fn remove_listener(&self, id: ListenerId) -> bool {
        // Check.typeOf.func("listener", listener) — statically guaranteed.
        type_of::func("listener", true);

        let mut removed = false;

        if self.invoking_listeners.get() {
            // During a raise, mark for removal after invocation finishes.
            let already_marked = self.to_remove.borrow().contains(&id);
            let in_listeners = self.listeners.borrow().iter().any(|e| e.id == id);
            if in_listeners && !already_marked {
                self.to_remove.borrow_mut().push(id);
                removed = true;
            } else {
                let mut to_add = self.to_add.borrow_mut();
                if let Some(pos) = to_add.iter().position(|e| e.id == id) {
                    to_add.remove(pos);
                    removed = true;
                }
            }
        } else {
            {
                let mut listeners = self.listeners.borrow_mut();
                if let Some(pos) = listeners.iter().position(|e| e.id == id) {
                    listeners.remove(pos);
                    removed = true;
                }
            }
            // Also drop pending additions registered during a nested raise.
            if !removed {
                let mut to_add = self.to_add.borrow_mut();
                if let Some(pos) = to_add.iter().position(|e| e.id == id) {
                    to_add.remove(pos);
                    removed = true;
                }
            }
        }

        if removed {
            self.listener_count.set(self.listener_count.get() - 1);
        }

        removed
    }

    /// Raises the event by calling each registered listener with all
    /// supplied arguments.
    ///
    /// Port of `Event.prototype.raiseEvent`.
    pub fn raise_event(&self, args: &A) {
        self.invoking_listeners.set(true);

        // The listener box is temporarily taken out of its entry (the entry
        // itself stays in the list, exactly like the JS `_listeners` array)
        // so that listeners may re-enter `add_listener` / `remove_listener`
        // (deferred via to_add / to_remove, exactly as in CesiumJS).
        let len = self.listeners.borrow().len();
        for i in 0..len {
            let taken = self
                .listeners
                .borrow_mut()
                .get_mut(i)
                .and_then(|e| e.listener.take());
            if let Some(mut listener) = taken {
                listener(args);
                if let Some(entry) = self.listeners.borrow_mut().get_mut(i) {
                    entry.listener = Some(listener);
                }
            }
        }

        self.invoking_listeners.set(false);

        // Actually add items marked for addition
        let to_add = std::mem::take(&mut *self.to_add.borrow_mut());
        for entry in to_add {
            self.listeners.borrow_mut().push(entry);
        }

        // Actually remove items marked for removal
        let to_remove = std::mem::take(&mut *self.to_remove.borrow_mut());
        for id in to_remove {
            let mut listeners = self.listeners.borrow_mut();
            if let Some(pos) = listeners.iter().position(|e| e.id == id) {
                listeners.remove(pos);
            }
        }
    }
}
