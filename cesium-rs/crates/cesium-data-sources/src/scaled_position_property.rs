//! Ported from `packages/engine/Source/DataSources/ScaledPositionProperty.js`.

use cesium_core::cartesian3::Cartesian3;
use crate::property::{Property, PropertyResult};
use crate::position_property::{PositionProperty, PositionReferenceFrame};

/// A position property that scales another position property.
pub struct ScaledPositionProperty {
    property: Box<dyn PositionProperty>,
    scale: f64,
}

impl ScaledPositionProperty {
    /// Creates a new scaled position property.
    pub fn new(property: Box<dyn PositionProperty>, scale: f64) -> Self {
        Self { property, scale }
    }
}

impl Property for ScaledPositionProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        match self.property.get_value(time) {
            PropertyResult::Position(x, y, z) => {
                PropertyResult::Position(x * self.scale, y * self.scale, z * self.scale)
            }
            other => other,
        }
    }

    fn is_constant(&self) -> bool { self.property.is_constant() }
    fn is_destroyed(&self) -> bool { self.property.is_destroyed() }
}

impl PositionProperty for ScaledPositionProperty {
    fn position_value<'a>(&self, time: f64, result: &'a mut Cartesian3) -> Option<&'a Cartesian3> {
        // Copy values first to avoid borrow conflict
        let mut temp = Cartesian3::new(0.0, 0.0, 0.0);
        if self.property.position_value(time, &mut temp).is_some() {
            result.x = temp.x * self.scale;
            result.y = temp.y * self.scale;
            result.z = temp.z * self.scale;
            Some(result)
        } else {
            None
        }
    }

    fn reference_frame(&self) -> PositionReferenceFrame { self.property.reference_frame() }
}
