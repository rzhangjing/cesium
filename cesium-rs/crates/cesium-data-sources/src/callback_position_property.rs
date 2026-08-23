//! Ported from `packages/engine/Source/DataSources/CallbackPositionProperty.js`.

use cesium_core::cartesian3::Cartesian3;
use crate::property::{Property, PropertyResult};
use crate::position_property::{PositionProperty, PositionReferenceFrame};

/// A position property whose value is computed by a callback function.
pub struct CallbackPositionProperty {
    callback: Box<dyn Fn(f64) -> Cartesian3 + Send + Sync>,
    reference_frame: PositionReferenceFrame,
    is_destroyed: bool,
}

impl CallbackPositionProperty {
    /// Creates a new callback position property.
    pub fn new(callback: Box<dyn Fn(f64) -> Cartesian3 + Send + Sync>, reference_frame: PositionReferenceFrame) -> Self {
        Self { callback, reference_frame, is_destroyed: false }
    }
}

impl Property for CallbackPositionProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        let pos = (self.callback)(time);
        PropertyResult::Position(pos.x, pos.y, pos.z)
    }

    fn is_constant(&self) -> bool { false }
    fn is_destroyed(&self) -> bool { self.is_destroyed }
}

impl PositionProperty for CallbackPositionProperty {
    fn position_value<'a>(&self, time: f64, result: &'a mut Cartesian3) -> Option<&'a Cartesian3> {
        let pos = (self.callback)(time);
        result.x = pos.x;
        result.y = pos.y;
        result.z = pos.z;
        Some(result)
    }

    fn reference_frame(&self) -> PositionReferenceFrame { self.reference_frame }
}
