//! Ported from `packages/engine/Source/DataSources/SampledProperty.js`.

use crate::property::{Property, PropertyResult};

/// A property whose value is interpolated from a set of samples.
pub struct SampledProperty {
    /// The type of property (Number, Position, etc.).
    pub type_name: String,
}

impl SampledProperty {
    /// Creates a new sampled property.
    pub fn new(type_name: &str) -> Self {
        Self { type_name: type_name.to_string() }
    }

    /// Adds a sample to the property.
    pub fn add_sample(&mut self, _time: f64, _value: PropertyResult) {
        // DEVIATION: requires sample storage and interpolation
    }
}

impl Property for SampledProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        // DEVIATION: requires interpolation
        PropertyResult::None
    }

    fn is_constant(&self) -> bool { false }
    fn is_destroyed(&self) -> bool { false }
}
