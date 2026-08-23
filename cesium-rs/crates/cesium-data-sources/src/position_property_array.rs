//! Ported from `packages/engine/Source/DataSources/PositionPropertyArray.js`.

use cesium_core::cartesian3::Cartesian3;
use crate::property::{Property, PropertyResult};

/// A property that represents an array of position values.
pub struct PositionPropertyArray {
    values: Vec<Cartesian3>,
}

impl PositionPropertyArray {
    /// Creates a new position property array.
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// Sets the values.
    pub fn set_values(&mut self, values: Vec<Cartesian3>) {
        self.values = values;
    }

    /// Returns the values.
    pub fn values(&self) -> &[Cartesian3] { &self.values }
}

impl Default for PositionPropertyArray {
    fn default() -> Self { Self::new() }
}

impl Property for PositionPropertyArray {
    fn get_value(&self, _time: f64) -> PropertyResult {
        // DEVIATION: Would need a PropertyResult variant for arrays
        PropertyResult::None
    }

    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }
}
