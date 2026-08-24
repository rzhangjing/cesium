//! Rust analogue of the Knockout.js observables used by CesiumJS widgets.
//!
//! In CesiumJS, widget ViewModels use Knockout observables
//! (`knockout.observable`, `knockout.track`) for MVVM data binding.
//! The widgets port models those observables with [`ObservableCell`]: a
//! cheap-to-clone handle over shared interior-mutable state with
//! notify-on-set subscription semantics.
//!
//! DEVIATION: knockout computed observables (`knockout.computed`,
//! `knockout.defineProperty`) are modeled as plain computed accessor
//! methods on the view models (evaluated on read) instead of a reactive
//! dependency graph; see `docs/deviations.md`.

use std::cell::RefCell;
use std::rc::Rc;

struct ObservableInner<T> {
    value: T,
    /// Entries are `None` only while the subscriber is currently being
    /// invoked (the box is taken out so that subscribers may re-enter
    /// `set`/`subscribe` on the same observable without a RefCell
    /// reentrancy panic — mirrors `cesium_core::Event::raise_event`).
    subscribers: Vec<Option<Box<dyn FnMut(&T)>>>,
}

/// A shared observable value, replacing `knockout.observable(value)`.
///
/// Clones share the same underlying value, mirroring the reference
/// semantics of JS observables (e.g. `knockout.getObservable(this, ...)`
/// handing the same observable to `createCommand`).
pub struct ObservableCell<T> {
    inner: Rc<RefCell<ObservableInner<T>>>,
}

impl<T> ObservableCell<T> {
    /// Creates a new observable with the given initial value.
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ObservableInner {
                value,
                subscribers: Vec::new(),
            })),
        }
    }

    /// Gets the current value.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.inner.borrow().value.clone()
    }

    /// Notifies all subscribers with `value`, taking each subscriber box
    /// out of the list for the duration of its invocation so subscribers
    /// may re-enter `set`/`subscribe` without a RefCell reentrancy panic
    /// (e.g. the write-back `synchronize` loops of `ClockViewModel`).
    fn notify(&self, value: T) {
        let len = self.inner.borrow().subscribers.len();
        for i in 0..len {
            let taken = self
                .inner
                .borrow_mut()
                .subscribers
                .get_mut(i)
                .and_then(|slot| slot.take());
            if let Some(mut subscriber) = taken {
                subscriber(&value);
                if let Some(slot) = self.inner.borrow_mut().subscribers.get_mut(i) {
                    *slot = Some(subscriber);
                }
            }
        }
    }

    /// Sets the value and notifies subscribers, mirroring the write
    /// semantics of a knockout observable.
    pub fn set(&self, value: T)
    where
        T: Clone,
    {
        self.inner.borrow_mut().value = value;
        // Clone into a local before notifying so the borrow is released
        // before subscribers run (subscribers may re-enter `set`).
        let value = self.inner.borrow().value.clone();
        self.notify(value);
    }

    /// Subscribes to value changes; the listener is invoked with the new
    /// value after each [`ObservableCell::set`].
    pub fn subscribe(&self, listener: impl FnMut(&T) + 'static) {
        self.inner
            .borrow_mut()
            .subscribers
            .push(Some(Box::new(listener)));
    }

    /// Sets the value and notifies subscribers only when `equals` reports
    /// a change, mirroring knockout observables with a custom
    /// `equalityComparer` (e.g. `startTime.equalityComparer =
    /// JulianDate.equals`).
    pub fn set_with_comparer(&self, value: T, equals: impl FnOnce(&T, &T) -> bool)
    where
        T: Clone,
    {
        {
            let mut inner = self.inner.borrow_mut();
            if equals(&inner.value, &value) {
                return;
            }
            inner.value = value;
        }
        // Clone into a local before notifying so the borrow is released
        // before subscribers run (subscribers may re-enter `set`).
        let value = self.inner.borrow().value.clone();
        self.notify(value);
    }
}

impl<T> Clone for ObservableCell<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}
