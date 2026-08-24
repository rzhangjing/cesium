//! Ported from `packages/engine/Source/Core/GeometryPipeline.js`
//! (section: fitToUnsignedShortIndices).

use std::collections::HashMap;

use crate::developer_error::throw_developer_error;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::math::CesiumMath;
use crate::primitive_type::PrimitiveType;

fn copy_attributes_descriptions(
    attributes: &HashMap<String, GeometryAttribute>,
) -> HashMap<String, GeometryAttribute> {
    let mut new_attributes = HashMap::new();
    for (attribute, attr) in attributes {
        new_attributes.insert(
            attribute.clone(),
            GeometryAttribute::new(
                attr.component_datatype,
                attr.components_per_attribute,
                attr.normalize,
                Vec::new(),
            ),
        );
    }
    new_attributes
}

fn copy_vertex(
    destination_attributes: &mut HashMap<String, GeometryAttribute>,
    source_attributes: &HashMap<String, GeometryAttribute>,
    index: usize,
) {
    let names: Vec<String> = source_attributes.keys().cloned().collect();
    for attribute in names {
        let attr = &source_attributes[&attribute];
        let components = attr.components_per_attribute as usize;
        let dest = destination_attributes.get_mut(&attribute).unwrap();
        for k in 0..components {
            dest.values.push(attr.values[index * components + k]);
        }
    }
}

/// Splits a geometry into multiple geometries, if necessary, to ensure that
/// indices in the `indices` fit into unsigned shorts. This is used to meet the
/// WebGL requirements when unsigned int indices are not supported.
///
/// If the geometry does not have any `indices`, this function has no effect.
///
/// Port of `GeometryPipeline.fitToUnsignedShortIndices(geometry)`.
///
/// # Panics (debug)
/// - If `primitive_type` is not TRIANGLES, LINES or POINTS (when indices are
///   defined).
pub fn fit_to_unsigned_short_indices(geometry: &Geometry) -> Vec<Geometry> {
    if cfg!(debug_assertions) {
        if geometry.indices.is_some()
            && geometry.primitive_type != PrimitiveType::Triangles
            && geometry.primitive_type != PrimitiveType::Lines
            && geometry.primitive_type != PrimitiveType::Points
        {
            throw_developer_error(
                "geometry.primitiveType must equal to PrimitiveType.TRIANGLES, PrimitiveType.LINES, or PrimitiveType.POINTS.",
            );
        }
    }

    let mut geometries: Vec<Geometry> = Vec::new();

    // If there's an index list and more than 64K attributes, it is possible
    // that some indices are outside the range of unsigned short [0, 64K - 1]
    let number_of_vertices = geometry.compute_number_of_vertices().unwrap_or(0);
    let original_indices: Option<&IndexStorage> = geometry.indices.as_ref();
    if original_indices.is_some()
        && (number_of_vertices as f64) >= CesiumMath::SIXTY_FOUR_KILOBYTES
    {
        let original_indices = original_indices.unwrap();
        let mut old_to_new_index: Vec<Option<u32>> = vec![None; number_of_vertices];
        let mut new_indices: Vec<u32> = Vec::new();
        let mut current_index: u32 = 0;
        let mut new_attributes = copy_attributes_descriptions(&geometry.attributes);

        let number_of_indices = original_indices.len();

        let indices_per_primitive: usize =
            if geometry.primitive_type == PrimitiveType::Triangles {
                3
            } else if geometry.primitive_type == PrimitiveType::Lines {
                2
            } else {
                1
            };

        for j in (0..number_of_indices).step_by(indices_per_primitive) {
            for k in 0..indices_per_primitive {
                let x = read_index(original_indices, j + k) as usize;
                let i = old_to_new_index[x];
                let i = match i {
                    Some(i) => i,
                    None => {
                        let fresh = current_index;
                        current_index += 1;
                        old_to_new_index[x] = Some(fresh);
                        copy_vertex(&mut new_attributes, &geometry.attributes, x);
                        fresh
                    }
                };
                new_indices.push(i);
            }

            if (current_index as f64) + (indices_per_primitive as f64)
                >= CesiumMath::SIXTY_FOUR_KILOBYTES
            {
                geometries.push(Geometry::with_all(
                    new_attributes.clone(),
                    Some(to_storage(&new_indices, current_index as usize)),
                    Some(geometry.primitive_type),
                    geometry.bounding_sphere.clone(),
                    geometry.geometry_type,
                    geometry.bounding_sphere_cv.clone(),
                    geometry.offset_attribute.clone(),
                ));

                // Reset for next vertex-array
                old_to_new_index = vec![None; number_of_vertices];
                new_indices = Vec::new();
                current_index = 0;
                new_attributes = copy_attributes_descriptions(&geometry.attributes);
            }
        }

        if !new_indices.is_empty() {
            geometries.push(Geometry::with_all(
                new_attributes,
                Some(to_storage(&new_indices, current_index as usize)),
                Some(geometry.primitive_type),
                geometry.bounding_sphere.clone(),
                geometry.geometry_type,
                geometry.bounding_sphere_cv.clone(),
                geometry.offset_attribute.clone(),
            ));
        }
    } else {
        // No need to split into multiple geometries.
        // DEVIATION: JS pushes the same geometry object reference; Rust clones.
        geometries.push(geometry.clone());
    }

    geometries
}

fn read_index(storage: &IndexStorage, index: usize) -> u32 {
    match storage {
        IndexStorage::U16(v) => v[index] as u32,
        IndexStorage::U32(v) => v[index],
    }
}

fn to_storage(indices: &[u32], number_of_vertices: usize) -> IndexStorage {
    let mut storage = IndexDatatype::create_typed_array(number_of_vertices, indices.len());
    match &mut storage {
        IndexStorage::U16(v) => {
            for (i, &value) in indices.iter().enumerate() {
                v[i] = value as u16;
            }
        }
        IndexStorage::U32(v) => v.copy_from_slice(indices),
    }
    storage
}
