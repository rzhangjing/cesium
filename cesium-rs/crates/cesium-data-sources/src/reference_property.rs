//! Ported from `packages/engine/Source/DataSources/ReferenceProperty.js`.

use crate::property::{Property, PropertyResult};

/// A property that references another property on a different entity.
///
/// Reference properties allow one entity to reference and use the
/// property values of another entity.
pub struct ReferenceProperty {
    /// The ID of the referenced entity.
    pub target_id: String,
    /// The property name on the referenced entity.
    pub property_name: String,
}

impl ReferenceProperty {
    /// Creates a new reference property.
    pub fn new(target_id: &str, property_name: &str) -> Self {
        Self {
            target_id: target_id.to_string(),
            property_name: property_name.to_string(),
        }
    }
}

impl Property for ReferenceProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        // DEVIATION: Requires entity collection lookup to resolve reference
        PropertyResult::None
    }

    fn is_constant(&self) -> bool { false }
    fn is_destroyed(&self) -> bool { false }
}
