//! Ported from `packages/engine/Source/DataSources/CompositePositionProperty.js`.
//!
//! A [`CompositeProperty`] which is also a [`PositionProperty`].

use std::rc::Rc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::event::{Event, RemoveCallback};

use crate::composite_property::CompositeProperty;
use crate::position_property::{PositionProperty, PositionReferenceFrame};
use crate::property::{Property, PropertyResult};

/// A [`CompositeProperty`] which is also a [`PositionProperty`].
pub struct CompositePositionProperty {
    /// Port of `_referenceFrame` (the "preferred" reference frame; each
    /// inner position property has its own frame).
    reference_frame: PositionReferenceFrame,
    composite: CompositeProperty,
    definition_changed: Rc<Event<()>>,
    /// Subscription forwarding the inner composite's `definitionChanged`
    /// onto this property's own event (held so it is dropped with self).
    _composite_subscription: Option<RemoveCallback<()>>,
}

impl CompositePositionProperty {
    /// Port of `new CompositePositionProperty(referenceFrame)` with
    /// `referenceFrame ?? ReferenceFrame.FIXED`.
    pub fn new(reference_frame: Option<PositionReferenceFrame>) -> Self {
        let composite = CompositeProperty::new();
        let definition_changed = Rc::new(Event::new());

        // Port of `this._composite.definitionChanged.addEventListener(
        // CompositePositionProperty.prototype._raiseDefinitionChanged, this)`.
        let raised = Rc::clone(&definition_changed);
        let subscription = composite
            .definition_changed()
            .map(|event| event.add_listener(move |_| raised.raise_event(&())));

        Self {
            reference_frame: reference_frame.unwrap_or(PositionReferenceFrame::Fixed),
            composite,
            definition_changed,
            _composite_subscription: subscription,
        }
    }

    /// Port of the `intervals` getter.
    pub fn intervals(
        &self,
    ) -> &crate::composite_intervals::CompositeIntervalCollection {
        self.composite.intervals()
    }

    /// Mutable access to the interval collection (mirrors in-place
    /// `addInterval` usage on `this._composite.intervals`).
    pub fn intervals_mut(
        &mut self,
    ) -> &mut crate::composite_intervals::CompositeIntervalCollection {
        self.composite.intervals_mut()
    }

    /// Accessor for the inner [`CompositeProperty`] (JS `this._composite`).
    pub fn composite(&self) -> &CompositeProperty {
        &self.composite
    }

    /// Port of the `referenceFrame` setter (plain assignment; the JS setter
    /// does not raise `definitionChanged`).
    pub fn set_reference_frame(&mut self, value: PositionReferenceFrame) {
        self.reference_frame = value;
    }

    /// Port of `getValueInReferenceFrame(time, referenceFrame, result)`:
    /// evaluates the position of the interval containing `time` in the
    /// requested reference frame, or `None` outside all intervals.
    ///
    /// DEVIATION: CesiumJS calls `innerProperty.getValueInReferenceFrame`
    /// via duck typing; the Rust port dispatches through
    /// [`Property::as_position_property`] and the
    /// [`PositionProperty::position_value`] of the inner property, which
    /// (mirroring the other position property ports) returns the value in
    /// the fixed frame. Conversions from an `Inertial` inner frame are
    /// applied when the inner property reports its own frame.
    pub fn get_value_in_reference_frame<'a>(
        &self,
        time: f64,
        reference_frame: PositionReferenceFrame,
        result: &'a mut Cartesian3,
    ) -> Option<&'a Cartesian3> {
        let inner = self
            .composite
            .intervals()
            .find_data_for_interval_containing_date(time)?;
        let inner_position = inner.as_position_property()?;
        let mut scratch = Cartesian3::ZERO;
        let value = inner_position.position_value(time, &mut scratch)?;
        let value = *value;
        let inner_frame = inner_position.reference_frame();
        if inner_frame == reference_frame {
            *result = value;
            return Some(result);
        }
        crate::position_property::convert_to_reference_frame(
            time,
            &value,
            inner_frame,
            reference_frame,
            result,
        )
        .map(|r| &*r)
    }

    /// Port of `equals(other)` for two [`CompositePositionProperty`]
    /// instances (mirrors `this._referenceFrame === other._referenceFrame
    /// && this._composite.equals(other._composite, Property.equals)`).
    pub fn equals_composite_position(&self, other: &CompositePositionProperty) -> bool {
        self.reference_frame == other.reference_frame
            && self.composite.equals_composite(&other.composite)
    }
}

impl Default for CompositePositionProperty {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Property for CompositePositionProperty {
    /// Port of `getValue(time, result)`: delegates to
    /// `getValueInReferenceFrame(time, ReferenceFrame.FIXED, result)`.
    fn get_value(&self, time: f64) -> PropertyResult {
        let mut result = Cartesian3::ZERO;
        match self.get_value_in_reference_frame(time, PositionReferenceFrame::Fixed, &mut result)
        {
            Some(value) => PropertyResult::Cartesian3(value.x, value.y, value.z),
            None => PropertyResult::None,
        }
    }

    fn is_constant(&self) -> bool {
        // JS: `this._composite.isConstant` (no intervals => constant).
        self.composite.is_constant()
    }

    fn is_destroyed(&self) -> bool {
        false
    }

    fn equals(&self, other: &dyn Property) -> bool {
        other
            .as_any()
            .and_then(|any| any.downcast_ref::<CompositePositionProperty>())
            .map(|other| self.equals_composite_position(other))
            .unwrap_or(false)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn as_position_property(
        &self,
    ) -> Option<&dyn crate::position_property::PositionProperty> {
        Some(self)
    }

    fn definition_changed(&self) -> Option<&Event<()>> {
        Some(&self.definition_changed)
    }
}

impl PositionProperty for CompositePositionProperty {
    /// Port of `getValue(time, result)` (fixed frame) as the
    /// [`PositionProperty`] value accessor.
    fn position_value<'a>(
        &self,
        time: f64,
        result: &'a mut Cartesian3,
    ) -> Option<&'a Cartesian3> {
        self.get_value_in_reference_frame(time, PositionReferenceFrame::Fixed, result)
    }

    fn reference_frame(&self) -> PositionReferenceFrame {
        self.reference_frame
    }

    fn get_value_in_reference_frame<'a>(
        &self,
        time: f64,
        reference_frame: PositionReferenceFrame,
        result: &'a mut Cartesian3,
    ) -> Option<&'a Cartesian3> {
        // Delegate to the inherent port of JS `getValueInReferenceFrame`
        // (dispatches into the interval's inner position property).
        self.get_value_in_reference_frame(time, reference_frame, result)
    }
}
