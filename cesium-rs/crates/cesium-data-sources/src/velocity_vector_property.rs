//! Ported from `packages/engine/Source/DataSources/VelocityVectorProperty.js`.

use cesium_core::cartesian3::Cartesian3;
use crate::property::{Property, PropertyResult};

/// A property that computes the velocity vector from a position property.
pub struct VelocityVectorProperty {
    is_destroyed: bool,
}

impl VelocityVectorProperty {
    /// Creates a new velocity vector property.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }
}

impl Default for VelocityVectorProperty {
    fn default() -> Self { Self::new() }
}

impl Property for VelocityVectorProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        // DEVIATION: Requires position property and numerical differentiation
        PropertyResult::None
    }

    fn is_constant(&self) -> bool { false }
    fn is_destroyed(&self) -> bool { self.is_destroyed }
}
