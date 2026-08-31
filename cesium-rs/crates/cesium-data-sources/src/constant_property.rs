//! Ported from `packages/engine/Source/DataSources/ConstantProperty.js`.

use std::cell::RefCell;

use cesium_core::event::Event;

use crate::property::{Property, PropertyResult};

/// A property with a constant value.
///
/// Port of `ConstantProperty`: a [`Property`] whose value does not change
/// with respect to simulation time. Setting a different value raises
/// [`definition_changed`](ConstantProperty::definition_changed).
pub struct ConstantProperty {
    /// DEVIATION: the value lives in a `RefCell` so that `setValue` keeps
    /// the CesiumJS shared-reference mutation semantics (`&self`) when the
    /// property is stored behind `Rc<dyn Property>` (e.g. as composite
    /// interval data).
    value: RefCell<PropertyResult>,
    definition_changed: Event<()>,
}

impl ConstantProperty {
    /// Creates a new constant property.
    ///
    /// Port of the `ConstantProperty` constructor: the initial value is
    /// installed through `setValue`, so a defined initial value raises the
    /// definition-changed event (no listeners can be attached yet, matching
    /// CesiumJS observable behavior).
    pub fn new(value: PropertyResult) -> Self {
        let property = Self {
            value: RefCell::new(PropertyResult::None),
            definition_changed: Event::new(),
        };
        property.set_value(value);
        property
    }

    /// Sets the value of the property.
    ///
    /// Port of `ConstantProperty.prototype.setValue`: the event is raised
    /// only when the new value differs from the current one (`oldValue !==
    /// value && (!hasEquals || !value.equals(oldValue))` in CesiumJS; the
    /// Rust value model compares by [`PropertyResult`] equality).
    pub fn set_value(&self, value: PropertyResult) {
        let mut current = self.value.borrow_mut();
        if *current != value {
            *current = value;
            drop(current);
            self.definition_changed.raise_event(&());
        }
    }

    /// Gets the event that is raised whenever the definition of this
    /// property changes.
    ///
    /// Port of the `definitionChanged` getter.
    pub fn definition_changed_event(&self) -> &Event<()> {
        &self.definition_changed
    }
}

impl Property for ConstantProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        self.value.borrow().clone()
    }

    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }

    fn equals(&self, other: &dyn Property) -> bool {
        if !other.is_constant() {
            return false;
        }
        let other_val = other.get_value(0.0);
        *self.value.borrow() == other_val
    }

    fn definition_changed(&self) -> Option<&Event<()>> {
        Some(&self.definition_changed)
    }
}
