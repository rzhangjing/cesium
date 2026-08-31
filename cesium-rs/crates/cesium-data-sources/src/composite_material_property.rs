//! Ported from `packages/engine/Source/DataSources/CompositeMaterialProperty.js`.
//!
//! A [`CompositeProperty`] which is also a [`MaterialProperty`].

use std::rc::Rc;

use cesium_core::event::{Event, RemoveCallback};

use crate::composite_property::CompositeProperty;
use crate::material_property::MaterialProperty;
use crate::property::{Property, PropertyResult};

/// A [`CompositeProperty`] which is also a [`MaterialProperty`].
pub struct CompositeMaterialProperty {
    composite: CompositeProperty,
    definition_changed: Rc<Event<()>>,
    /// Subscription forwarding the inner composite's `definitionChanged`
    /// onto this property's own event (held so it is dropped with self).
    _composite_subscription: Option<RemoveCallback<()>>,
}

impl CompositeMaterialProperty {
    /// Port of `new CompositeMaterialProperty()`.
    pub fn new() -> Self {
        let composite = CompositeProperty::new();
        let definition_changed = Rc::new(Event::new());

        // Port of `this._composite.definitionChanged.addEventListener(
        // CompositeMaterialProperty.prototype._raiseDefinitionChanged, this)`.
        let raised = Rc::clone(&definition_changed);
        let subscription = composite
            .definition_changed()
            .map(|event| event.add_listener(move |_| raised.raise_event(&())));

        Self {
            composite,
            definition_changed,
            _composite_subscription: subscription,
        }
    }

    /// Port of the `intervals` getter (JS exposes the composite's interval
    /// collection directly).
    pub fn intervals(
        &self,
    ) -> &crate::composite_intervals::CompositeIntervalCollection {
        self.composite.intervals()
    }

    /// Mutable access to the interval collection (mirrors in-place
    /// `addInterval` usage on `this._composite._intervals`).
    pub fn intervals_mut(
        &mut self,
    ) -> &mut crate::composite_intervals::CompositeIntervalCollection {
        self.composite.intervals_mut()
    }

    /// Accessor for the inner [`CompositeProperty`] (JS `this._composite`).
    pub fn composite(&self) -> &CompositeProperty {
        &self.composite
    }

    /// Port of `getType(time)`: returns the material type of the interval
    /// data property containing `time`, or `None` outside all intervals
    /// (JS `undefined`).
    ///
    /// DEVIATION: CesiumJS calls `innerProperty.getType(time)` directly via
    /// duck typing; the Rust port dispatches through
    /// [`Property::material_type_name`] (the time-independent
    /// [`MaterialProperty::type_name`] of the interval data).
    pub fn get_type_at(&self, time: f64) -> Option<&'static str> {
        let inner = self
            .composite
            .intervals()
            .find_data_for_interval_containing_date(time)?;
        inner.material_type_name()
    }

    /// Port of `getValue(time, result)`: evaluates the material value of
    /// the interval containing `time`, or `None` outside all intervals.
    pub fn get_value_option(&self, time: f64) -> Option<PropertyResult> {
        let inner = self
            .composite
            .intervals()
            .find_data_for_interval_containing_date(time)?;
        Some(inner.get_value(time))
    }

    /// Port of `equals(other)` for two [`CompositeMaterialProperty`]
    /// instances (mirrors `this._composite.equals(other._composite,
    /// Property.equals)`).
    pub fn equals_composite_material(&self, other: &CompositeMaterialProperty) -> bool {
        self.composite.equals_composite(&other.composite)
    }
}

impl Default for CompositeMaterialProperty {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialProperty for CompositeMaterialProperty {
    /// DEVIATION: CesiumJS `MaterialProperty` instances expose a
    /// time-dependent `getType(time)`; the Rust trait `type_name()` is
    /// static, so the composite reports `"Composite"`. Use
    /// [`CompositeMaterialProperty::get_type_at`] for the JS `getType`
    /// semantics.
    fn type_name(&self) -> &str {
        "Composite"
    }

    fn is_constant(&self) -> bool {
        // JS: `this._composite.isConstant` (no intervals => constant).
        self.composite.is_constant()
    }

    fn is_destroyed(&self) -> bool {
        false
    }
}

impl Property for CompositeMaterialProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        self.get_value_option(time).unwrap_or(PropertyResult::None)
    }

    fn is_constant(&self) -> bool {
        self.composite.is_constant()
    }

    fn is_destroyed(&self) -> bool {
        false
    }

    fn equals(&self, other: &dyn Property) -> bool {
        other
            .as_any()
            .and_then(|any| any.downcast_ref::<CompositeMaterialProperty>())
            .map(|other| self.equals_composite_material(other))
            .unwrap_or(false)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn material_type_name(&self) -> Option<&'static str> {
        Some("Composite")
    }

    fn definition_changed(&self) -> Option<&Event<()>> {
        Some(&self.definition_changed)
    }
}
