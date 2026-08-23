//! Ported from `packages/engine/Source/DataSources/PropertyArray.js`.

use crate::property::{Property, PropertyResult};

/// An array of property values that may vary over time.
pub struct PropertyArray {
    values: Vec<Box<dyn Property>>,
}

impl PropertyArray {
    /// Creates a new property array.
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// Returns the number of properties.
    pub fn len(&self) -> usize { self.values.len() }

    /// Returns whether the array is empty.
    pub fn is_empty(&self) -> bool { self.values.is_empty() }

    /// Gets a property by index.
    pub fn get(&self, index: usize) -> Option<&dyn Property> {
        self.values.get(index).map(|p| p.as_ref())
    }
}

impl Default for PropertyArray {
    fn default() -> Self { Self::new() }
}

impl Property for PropertyArray {
    fn get_value(&self, _time: f64) -> PropertyResult {
        PropertyResult::None
    }

    fn is_constant(&self) -> bool {
        self.values.iter().all(|v| v.is_constant())
    }
    fn is_destroyed(&self) -> bool { false }
}
