//! Ported from `packages/engine/Source/DataSources/TerrainOffsetProperty.js`.

use cesium_core::cartesian3::Cartesian3;
use crate::property::{Property, PropertyResult};
use crate::position_property::{PositionProperty, PositionReferenceFrame};

/// A position property that offsets another position property to account for terrain height.
///
/// This is used internally to adjust entity positions so they appear on the terrain surface.
pub struct TerrainOffsetProperty {
    position_property: Box<dyn PositionProperty>,
    is_destroyed: bool,
}

impl TerrainOffsetProperty {
    /// Creates a new terrain offset property.
    pub fn new(position_property: Box<dyn PositionProperty>) -> Self {
        Self { position_property, is_destroyed: false }
    }
}

impl Property for TerrainOffsetProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        self.position_property.get_value(time)
    }

    fn is_constant(&self) -> bool { self.position_property.is_constant() }
    fn is_destroyed(&self) -> bool { self.is_destroyed }

    fn as_position_property(
        &self,
    ) -> Option<&dyn crate::position_property::PositionProperty> {
        Some(self)
    }
}

impl PositionProperty for TerrainOffsetProperty {
    fn position_value<'a>(&self, time: f64, result: &'a mut Cartesian3) -> Option<&'a Cartesian3> {
        self.position_property.position_value(time, result)
    }

    fn reference_frame(&self) -> PositionReferenceFrame { self.position_property.reference_frame() }
}
