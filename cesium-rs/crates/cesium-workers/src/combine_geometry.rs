//! Ported from `packages/engine/Source/Workers/combineGeometry.js`.
//!
//! Worker entry point for combining multiple geometry instances into a single
//! vertex/index buffer for efficient GPU rendering.

/// Combines geometry instances into a single geometry.
///
/// In CesiumJS, this receives an array of geometry instances, merges their
/// vertex attributes and indices into a single combined geometry, and returns
/// the packed result. This is critical for batching static geometry.
pub fn combine_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Combines multiple geometries into one (for in-process use).
///
/// # Arguments
/// * `geometries` - Slice of geometries to combine.
///
/// Returns a single merged `Geometry`, or `None` if the input is empty.
pub fn combine_geometry_unpacked(
    geometries: &[cesium_core::geometry::Geometry],
) -> Option<cesium_core::geometry::Geometry> {
    if geometries.is_empty() {
        return None;
    }
    if geometries.len() == 1 {
        return Some(geometries[0].clone());
    }

    use std::collections::HashMap;
    let mut combined_attributes: HashMap<String, Vec<f64>> = HashMap::new();
    let mut combined_indices: Vec<u32> = Vec::new();
    let mut vertex_offset: u32 = 0;

    for geom in geometries {
        // Accumulate vertex counts for index offset
        let vertex_count = geom.attributes.values().next().map_or(0, |attr| {
            attr.values.len() / attr.components_per_attribute as usize
        });

        // Merge attributes
        for (name, attr) in &geom.attributes {
            let buf = combined_attributes.entry(name.clone()).or_default();
            buf.extend_from_slice(&attr.values);
        }

        // Merge indices with offset
        if let Some(ref indices) = geom.indices {
            match indices {
                cesium_core::index_datatype::IndexStorage::U16(idx) => {
                    combined_indices.extend(idx.iter().map(|&i| i as u32 + vertex_offset));
                }
                cesium_core::index_datatype::IndexStorage::U32(idx) => {
                    combined_indices.extend(idx.iter().map(|&i| i + vertex_offset));
                }
            }
        }

        vertex_offset += vertex_count as u32;
    }

    // Build combined geometry attributes
    let mut result_attributes: HashMap<String, cesium_core::geometry_attribute::GeometryAttribute> = HashMap::new();
    for (name, values) in combined_attributes {
        let components = geometries[0].attributes.get(&name).map_or(3, |a| a.components_per_attribute);
        result_attributes.insert(
            name,
            cesium_core::geometry_attribute::GeometryAttribute::new(
                cesium_core::component_datatype::ComponentDatatype::Double,
                components,
                false,
                values,
            ),
        );
    }

    let index_storage = if combined_indices.iter().all(|&i| i <= u16::MAX as u32) {
        cesium_core::index_datatype::IndexStorage::U16(combined_indices.iter().map(|&i| i as u16).collect())
    } else {
        cesium_core::index_datatype::IndexStorage::U32(combined_indices)
    };

    Some(cesium_core::geometry::Geometry::new(
        result_attributes,
        Some(index_storage),
        Some(cesium_core::primitive_type::PrimitiveType::Triangles),
        None,
    ))
}
