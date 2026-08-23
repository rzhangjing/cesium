//! Ported from `packages/engine/Source/DataSources/ConstantProperty.js`.

use crate::property::{Property, PropertyResult};

/// A property with a constant value.
pub struct ConstantProperty {
    value: PropertyResult,
}

impl ConstantProperty {
    /// Creates a new constant property.
    pub fn new(value: PropertyResult) -> Self {
        Self { value }
    }
}

impl Property for ConstantProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        self.value.clone()
    }

    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }

    fn equals(&self, other: &dyn Property) -> bool {
        if !other.is_constant() {
            return false;
        }
        let other_val = other.get_value(0.0);
        self.value == other_val
    }
}
