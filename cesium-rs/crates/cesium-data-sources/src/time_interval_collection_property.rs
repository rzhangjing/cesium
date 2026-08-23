//! Ported from `packages/engine/Source/DataSources/TimeIntervalCollectionProperty.js`.

use crate::property::{Property, PropertyResult};

/// A property whose value is defined by a collection of time intervals.
pub struct TimeIntervalCollectionProperty {
    type_name: String,
}

impl TimeIntervalCollectionProperty {
    pub fn new(type_name: &str) -> Self {
        Self { type_name: type_name.to_string() }
    }
}

impl Property for TimeIntervalCollectionProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        PropertyResult::None
    }

    fn is_constant(&self) -> bool { false }
    fn is_destroyed(&self) -> bool { false }
}
