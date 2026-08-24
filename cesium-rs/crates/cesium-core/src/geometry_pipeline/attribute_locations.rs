//! Ported from `packages/engine/Source/Core/GeometryPipeline.js`
//! (section: createAttributeLocations).

use std::collections::HashMap;

use crate::geometry::Geometry;

/// Creates an object that maps attribute names to unique locations (indices)
/// for matching vertex attributes and shader programs.
///
/// Port of `GeometryPipeline.createAttributeLocations(geometry)`.
pub fn create_attribute_locations(geometry: &Geometry) -> HashMap<String, u32> {
    // There can be a WebGL performance hit when attribute 0 is disabled, so
    // assign attribute locations to well-known attributes.
    const SEMANTICS: [&str; 14] = [
        "position",
        "positionHigh",
        "positionLow",
        // From VertexFormat.position - after 2D projection and high-precision encoding
        "position3DHigh",
        "position3DLow",
        "position2DHigh",
        "position2DLow",
        // From Primitive
        "pickColor",
        // From VertexFormat
        "normal",
        "st",
        "tangent",
        "bitangent",
        // For shadow volumes
        "extrudeDirection",
        // From compressing texture coordinates and normals
        // "compressedAttributes" is intentionally last (matches JS order).
        "compressedAttributes",
    ];

    let attributes = &geometry.attributes;
    let mut indices: HashMap<String, u32> = HashMap::new();
    let mut j: u32 = 0;

    // Attribute locations for well-known attributes
    for semantic in SEMANTICS.iter() {
        if attributes.contains_key(*semantic) {
            indices.insert((*semantic).to_string(), j);
            j += 1;
        }
    }

    // Locations for custom attributes.
    //
    // DEVIATION: JS iterates object keys in insertion order; `HashMap` order is
    // unspecified. Relative ordering only matters when several custom
    // attributes coexist, which does not affect rendering correctness.
    let mut custom_names: Vec<&String> = attributes
        .keys()
        .filter(|name| !indices.contains_key(*name))
        .collect();
    custom_names.sort();
    for name in custom_names {
        indices.insert(name.clone(), j);
        j += 1;
    }

    indices
}
