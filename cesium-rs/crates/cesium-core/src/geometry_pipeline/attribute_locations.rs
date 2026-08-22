//! `createAttributeLocations` – maps attribute names to shader locations.

use std::collections::HashMap;

use crate::geometry::Geometry;

/// Creates a mapping from attribute names to unique location indices.
pub fn create_attribute_locations(geometry: &Geometry) -> HashMap<String, usize> {
    let semantics = [
        "position", "positionHigh", "positionLow",
        "position3DHigh", "position3DLow", "position2DHigh", "position2DLow",
        "pickColor", "normal", "st", "tangent", "bitangent",
        "extrudeDirection", "compressedAttributes",
    ];

    let mut indices = HashMap::new();
    let mut j = 0;

    for &semantic in &semantics {
        if geometry.attributes.contains_key(semantic) {
            indices.insert(semantic.to_string(), j);
            j += 1;
        }
    }

    for name in geometry.attributes.keys() {
        if !indices.contains_key(name) {
            indices.insert(name.clone(), j);
            j += 1;
        }
    }

    indices
}
