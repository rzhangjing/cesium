//! Ported from `packages/engine/Source/DataSources/CompositeProperty.js`.
//!
//! A [`Property`] which is defined by a time interval collection, where the
//! data of each interval is another [`Property`] evaluated at the provided
//! time.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use cesium_core::event::{Event, RemoveCallback};

use crate::composite_intervals::CompositeIntervalCollection;
use crate::property::{property_equals, Property, PropertyResult};

/// Port of the module-level `subscribeAll(property, eventHelper,
/// definitionChanged, intervals)` helper: (re)subscribes the composite's
/// `definitionChanged` forwarding to every distinct interval data
/// property.
///
/// Each forwarding listener captures the collection `version` it was
/// installed under; once the collection changes (bumping the version) the
/// stale listeners stop forwarding even before the deferred resubscription
/// runs, mirroring the CesiumJS synchronous `eventHelper.removeAll()` +
/// re-subscribe inside `_intervalsChanged`.
fn subscribe_all(
    intervals: &CompositeIntervalCollection,
    event_helper_removals: &mut Vec<Box<dyn FnMut()>>,
    definition_changed: &Rc<Event<()>>,
    version: &Rc<Cell<u64>>,
) {
    // eventHelper.removeAll()
    for removal in event_helper_removals.drain(..) {
        let mut removal = removal;
        removal();
    }

    let subscribed_version = version.get();
    let mut items: Vec<Rc<dyn Property>> = Vec::new();
    for index in 0..intervals.length() {
        let Some(interval) = intervals.get(index) else {
            continue;
        };
        // JS `items.indexOf(interval.data) === -1` (reference identity).
        if items.iter().any(|item| Rc::ptr_eq(item, &interval.data)) {
            continue;
        }
        items.push(Rc::clone(&interval.data));

        if let Some(event) = interval.data.definition_changed() {
            let raised = Rc::clone(definition_changed);
            let current_version = Rc::clone(version);
            let remove = event.add_listener(move |_| {
                // Stale subscription (the collection changed since this
                // listener was installed): JS already removed it inside
                // `_intervalsChanged`; the deferred Rust resubscription
                // silences it instead.
                if current_version.get() == subscribed_version {
                    raised.raise_event(&());
                }
            });
            let id = remove.id();
            let data = Rc::clone(&interval.data);
            event_helper_removals.push(Box::new(move || {
                if let Some(event) = data.definition_changed() {
                    event.remove_listener(id);
                }
            }));
        }
    }
}

/// A [`Property`] which is defined by a time interval collection, where the
/// data of each interval is another property evaluated at the provided time.
pub struct CompositeProperty {
    intervals: CompositeIntervalCollection,
    /// EventHelper removal tokens (port of `_eventHelper`).
    event_helper_removals: RefCell<Vec<Box<dyn FnMut()>>>,
    definition_changed: Rc<Event<()>>,
    _intervals_subscription: Option<RemoveCallback<()>>,
    /// Set by the `intervals.changedEvent` subscription; the JS handler
    /// resubscribes synchronously, the Rust port resubscribes lazily at the
    /// next API entry point (see DEVIATION note on `definition_changed`).
    needs_resubscribe: Rc<Cell<bool>>,
    /// Bumped on every collection change so stale forwarding listeners
    /// (installed before the change) stop forwarding until the deferred
    /// resubscription reinstalls them.
    version: Rc<Cell<u64>>,
}

impl CompositeProperty {
    /// Port of `new CompositeProperty()`.
    pub fn new() -> Self {
        let intervals = CompositeIntervalCollection::new();
        let definition_changed = Rc::new(Event::new());
        let needs_resubscribe = Rc::new(Cell::new(false));
        let version = Rc::new(Cell::new(0));

        // Port of `this._intervals.changedEvent.addEventListener(
        // CompositeProperty.prototype._intervalsChanged, this)`.
        let flag = Rc::clone(&needs_resubscribe);
        let raised = Rc::clone(&definition_changed);
        let version_for_listener = Rc::clone(&version);
        let intervals_subscription = intervals.changed_event().add_listener(move |_| {
            version_for_listener.set(version_for_listener.get() + 1);
            flag.set(true);
            raised.raise_event(&());
        });

        Self {
            intervals,
            event_helper_removals: RefCell::new(Vec::new()),
            definition_changed,
            _intervals_subscription: Some(intervals_subscription),
            needs_resubscribe,
            version,
        }
    }

    /// Port of the `intervals` getter.
    pub fn intervals(&self) -> &CompositeIntervalCollection {
        &self.intervals
    }

    /// Mutable access to the interval collection (the JS exposes `intervals`
    /// for in-place `addInterval` calls). Refreshes the inner-property
    /// subscriptions that were invalidated by previous collection changes.
    pub fn intervals_mut(&mut self) -> &mut CompositeIntervalCollection {
        self.ensure_subscribed();
        &mut self.intervals
    }

    /// Port of `_intervalsChanged`'s `subscribeAll` half (deferred; see the
    /// DEVIATION note on [`Property::definition_changed`]).
    pub fn ensure_subscribed(&self) {
        if self.needs_resubscribe.get() {
            subscribe_all(
                &self.intervals,
                &mut self.event_helper_removals.borrow_mut(),
                &self.definition_changed,
                &self.version,
            );
            self.needs_resubscribe.set(false);
        }
    }

    /// Port of `getValue(time)`: evaluates the property of the interval
    /// containing `time`, or `None` outside all intervals.
    pub fn get_value_option(&self, time: f64) -> Option<PropertyResult> {
        self.ensure_subscribed();
        let inner = self
            .intervals
            .find_data_for_interval_containing_date(time)?;
        Some(inner.get_value(time))
    }

    /// Port of `equals(other)` for two [`CompositeProperty`] instances
    /// (mirrors `intervals.equals(other._intervals, Property.equals)`).
    pub fn equals_composite(&self, other: &CompositeProperty) -> bool {
        let comparer = |left: &dyn Property, right: &dyn Property| property_equals(left, right);
        self.intervals.equals(&other.intervals, Some(&comparer))
    }
}

impl Default for CompositeProperty {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CompositeProperty {
    fn drop(&mut self) {
        // eventHelper.removeAll(): unsubscribe from all inner properties.
        for removal in self.event_helper_removals.borrow_mut().drain(..) {
            let mut removal = removal;
            removal();
        }
    }
}

impl Property for CompositeProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        self.get_value_option(time).unwrap_or(PropertyResult::None)
    }

    fn is_constant(&self) -> bool {
        // JS: a composite with no intervals is constant.
        self.intervals.is_empty()
    }

    fn is_destroyed(&self) -> bool {
        false
    }

    fn equals(&self, other: &dyn Property) -> bool {
        other
            .as_any()
            .and_then(|any| any.downcast_ref::<CompositeProperty>())
            .map(|other| self.equals_composite(other))
            .unwrap_or(false)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    /// DEVIATION: CesiumJS resubscribes to the interval data properties
    /// synchronously inside the `changedEvent` handler; the Rust port defers
    /// the resubscription to the next API entry point (`get_value` /
    /// `intervals_mut` / `ensure_subscribed`) because the handler cannot
    /// borrow the owning struct. `definitionChanged` is raised with the same
    /// timing as CesiumJS.
    fn definition_changed(&self) -> Option<&Event<()>> {
        Some(&self.definition_changed)
    }
}
