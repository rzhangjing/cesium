//! Ported from `packages/engine/Source/Core/GeometryPipeline.js`
//! (section: reorderForPreVertexCache / reorderForPostVertexCache).

use crate::geometry::Geometry;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::primitive_type::PrimitiveType;
use crate::tipsify::{tipsify, TipsifyOptions};

/// Reorders a geometry's attributes and `indices` to achieve better
/// performance from the GPU's pre-vertex-shader cache.
///
/// Port of `GeometryPipeline.reorderForPreVertexCache(geometry)`.
pub fn reorder_for_pre_vertex_cache(geometry: &mut Geometry) {
    let num_vertices = geometry.compute_number_of_vertices().unwrap_or(0);

    let indices = match geometry.indices.take() {
        Some(indices) => indices,
        None => return,
    };

    // Int32Array(numVertices) filled with -1
    let mut index_cross_reference_old_to_new = vec![-1i32; num_vertices];

    // Construct cross reference and reorder indices
    let num_indices = indices.len();
    let mut indices_out: Vec<u32> = vec![0; num_indices];

    let mut into_indices_in = 0usize;
    let mut into_indices_out = 0usize;
    let mut next_index: u32 = 0;
    while into_indices_in < num_indices {
        let old_index = read_index(&indices, into_indices_in) as i32;
        let temp_index = index_cross_reference_old_to_new[old_index as usize];
        if temp_index != -1 {
            indices_out[into_indices_out] = temp_index as u32;
        } else {
            index_cross_reference_old_to_new[old_index as usize] = next_index as i32;
            indices_out[into_indices_out] = next_index;
            next_index += 1;
        }
        into_indices_in += 1;
        into_indices_out += 1;
    }
    // DEVIATION: JS creates the output typed array from `numVertices`
    // (the pre-reorder count); Rust picks the U16/U32 width the same way.
    geometry.indices = Some(IndexDatatype::create_typed_array(num_vertices, num_indices));
    if let Some(ref mut storage) = geometry.indices {
        for (j, &value) in indices_out.iter().enumerate() {
            write_index(storage, j, value);
        }
    }

    // Reorder attributes
    let attribute_names: Vec<String> = geometry.attributes.keys().cloned().collect();
    for property in attribute_names {
        let attribute = match geometry.attributes.get(&property) {
            Some(a) if !a.values.is_empty() => a.clone(),
            _ => continue,
        };
        let elements_in = &attribute.values;
        let num_components = attribute.components_per_attribute as usize;
        let mut elements_out = vec![0.0f64; next_index as usize * num_components];
        for into_elements_in in 0..num_vertices {
            let temp = index_cross_reference_old_to_new[into_elements_in];
            if temp != -1 {
                let temp = temp as usize;
                for j in 0..num_components {
                    elements_out[num_components * temp + j] =
                        elements_in[num_components * into_elements_in + j];
                }
            }
        }
        if let Some(attr) = geometry.attributes.get_mut(&property) {
            attr.values = elements_out;
        }
    }
}

/// Reorders a geometry's `indices` to achieve better performance from the
/// GPU's post vertex-shader cache by using the Tipsify algorithm. If the
/// geometry `primitive_type` is not `TRIANGLES` or the geometry does not have
/// an `indices`, this function has no effect.
///
/// Port of `GeometryPipeline.reorderForPostVertexCache(geometry, cacheCapacity)`.
pub fn reorder_for_post_vertex_cache(geometry: &mut Geometry, cache_capacity: Option<u32>) {
    if geometry.primitive_type != PrimitiveType::Triangles {
        return;
    }
    let indices = match geometry.indices.take() {
        Some(indices) => indices,
        None => return,
    };

    let num_indices = indices.len();
    let mut maximum_index: u32 = 0;
    for j in 0..num_indices {
        let index = read_index(&indices, j);
        if index > maximum_index {
            maximum_index = index;
        }
    }
    let indices_vec: Vec<u32> = (0..num_indices).map(|j| read_index(&indices, j)).collect();
    let result = tipsify(&TipsifyOptions {
        indices: &indices_vec,
        maximum_index: Some(maximum_index),
        cache_size: cache_capacity.unwrap_or(24),
    });
    // DEVIATION: JS keeps the original typed-array width; Rust picks U16/U32
    // from the vertex count, matching `IndexDatatype.createTypedArray`.
    geometry.indices = Some(IndexDatatype::create_typed_array(
        maximum_index as usize + 1,
        result.len(),
    ));
    if let Some(ref mut storage) = geometry.indices {
        for (j, &value) in result.iter().enumerate() {
            write_index(storage, j, value);
        }
    }
}

fn read_index(storage: &IndexStorage, index: usize) -> u32 {
    match storage {
        IndexStorage::U16(v) => v[index] as u32,
        IndexStorage::U32(v) => v[index],
    }
}

fn write_index(storage: &mut IndexStorage, index: usize, value: u32) {
    match storage {
        IndexStorage::U16(v) => v[index] = value as u16,
        IndexStorage::U32(v) => v[index] = value,
    }
}
