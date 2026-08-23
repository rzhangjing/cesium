//! Ported from `packages/engine/Source/DataSources/CompositeProperty.js`.

use crate::property::{Property, PropertyResult};

/// A property that composes multiple properties, selecting the first
/// non-undefined value.
pub struct CompositeProperty {
    properties: Vec<Box<dyn Property>>,
}

impl CompositeProperty {
    pub fn new() -> Self {
        Self { properties: Vec::new() }
    }

    pub fn add(&mut self, property: Box<dyn Property>) {
        self.properties.push(property);
    }
}

impl Default for CompositeProperty {
    fn default() -> Self { Self::new() }
}

impl Property for CompositeProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        for prop in &self.properties {
            let val = prop.get_value(time);
            if !matches!(val, PropertyResult::None) {
                return val;
            }
        }
        PropertyResult::None
    }

    fn is_constant(&self) -> bool {
        self.properties.iter().all(|p| p.is_constant())
    }

    fn is_destroyed(&self) -> bool { false }
}
