//! Ported from `packages/engine/Source/DataSources/ConstantPositionProperty.js`.

use cesium_core::cartesian3::Cartesian3;
use crate::property::{Property, PropertyResult};
use crate::position_property::{PositionProperty, PositionReferenceFrame};

/// A position property with a constant value.
pub struct ConstantPositionProperty {
    value: Cartesian3,
    reference_frame: PositionReferenceFrame,
}

impl ConstantPositionProperty {
    /// Creates a new constant position property.
    pub fn new(value: Cartesian3) -> Self {
        Self { value, reference_frame: PositionReferenceFrame::Fixed }
    }

    /// Sets the value of this property.
    pub fn set_value(&mut self, value: Cartesian3) {
        self.value = value;
    }

    /// Sets the reference frame.
    pub fn set_reference_frame(&mut self, reference_frame: PositionReferenceFrame) {
        self.reference_frame = reference_frame;
    }
}

impl Property for ConstantPositionProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        PropertyResult::Position(self.value.x, self.value.y, self.value.z)
    }

    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }
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
