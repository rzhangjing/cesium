//! Ported from `packages/engine/Source/DataSources/ConstantPositionProperty.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::event::Event;
use crate::property::{Property, PropertyResult};
use crate::position_property::{PositionProperty, PositionReferenceFrame};

/// A position property with a constant value.
pub struct ConstantPositionProperty {
    value: Cartesian3,
    reference_frame: PositionReferenceFrame,
    definition_changed: Event<()>,
}

impl ConstantPositionProperty {
    /// Creates a new constant position property.
    pub fn new(value: Cartesian3) -> Self {
        Self {
            value,
            reference_frame: PositionReferenceFrame::Fixed,
            definition_changed: Event::new(),
        }
    }

    /// Sets the value of this property.
    ///
    /// Port of `ConstantPositionProperty.prototype.setValue`: raises
    /// `definitionChanged` only when the value actually changes
    /// (`!Cartesian3.equals(value, this._value)` in CesiumJS).
    pub fn set_value(&mut self, value: Cartesian3) {
        if self.value != value {
            self.value = value;
            self.definition_changed.raise_event(&());
        }
    }

    /// Sets the reference frame.
    ///
    /// Port of `ConstantPositionProperty.prototype.setReferenceFrame`:
    /// raises `definitionChanged` only when the reference frame changes.
    pub fn set_reference_frame(&mut self, reference_frame: PositionReferenceFrame) {
        if self.reference_frame != reference_frame {
            self.reference_frame = reference_frame;
            self.definition_changed.raise_event(&());
        }
    }

    /// Gets the event that is raised whenever the definition of this
    /// property changes (port of the `definitionChanged` getter).
    pub fn definition_changed_event(&self) -> &Event<()> {
        &self.definition_changed
    }
}

impl Property for ConstantPositionProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        PropertyResult::Position(self.value.x, self.value.y, self.value.z)
    }

    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }

    /// Port of `ConstantPositionProperty.prototype.equals`: compares by
    /// `Cartesian3.equals` of the value and the reference frame.
    fn equals(&self, other: &dyn Property) -> bool {
        other
            .as_any()
            .and_then(|any| any.downcast_ref::<ConstantPositionProperty>())
            .map(|other| {
                self.value == other.value && self.reference_frame == other.reference_frame
            })
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

impl PositionProperty for ConstantPositionProperty {
    fn position_value<'a>(&self, _time: f64, result: &'a mut Cartesian3) -> Option<&'a Cartesian3> {
        result.x = self.value.x;
        result.y = self.value.y;
        result.z = self.value.z;
        Some(result)
    }

    fn reference_frame(&self) -> PositionReferenceFrame { self.reference_frame }
}
