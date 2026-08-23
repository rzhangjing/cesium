//! Ported from `packages/engine/Source/DataSources/VelocityOrientationProperty.js`.

use cesium_core::quaternion::Quaternion;
use crate::property::{Property, PropertyResult};

/// A property that computes orientation from the velocity of a position property.
///
/// This is useful for orienting entities (like aircraft) along their path of travel.
pub struct VelocityOrientationProperty {
    is_destroyed: bool,
}

impl VelocityOrientationProperty {
    /// Creates a new velocity orientation property.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }
}

impl Default for VelocityOrientationProperty {
    fn default() -> Self { Self::new() }
}

impl Property for VelocityOrientationProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        // DEVIATION: Requires position property and velocity computation
        PropertyResult::None
    }

    fn is_constant(&self) -> bool { false }
    fn is_destroyed(&self) -> bool { self.is_destroyed }
}
