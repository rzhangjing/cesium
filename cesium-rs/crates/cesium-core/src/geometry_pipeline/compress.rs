//! Ported from `packages/engine/Source/Core/GeometryPipeline.js`
//! (section: compressVertices).

use crate::attribute_compression::AttributeCompression;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;

/// Compresses and packs geometry normal attribute values to save memory.
///
/// Port of `GeometryPipeline.compressVertices(geometry)`.
pub fn compress_vertices(geometry: &mut Geometry) {
    // Only shadow volumes use extrudeDirection, and shadow volumes use
    // vertexFormat: POSITION_ONLY so we don't need to check other attributes.
    let extrude_directions = geometry
        .attributes
        .get("extrudeDirection")
        .map(|a| a.values.clone());
    if let Some(extrude_directions) = extrude_directions {
        let num_vertices = extrude_directions.len() / 3;
        let mut compressed_directions = vec![0.0f64; num_vertices * 2];

        let mut i2 = 0usize;
        let mut to_encode1 = Cartesian3::ZERO;
        let mut encode_result2 = Cartesian2::ZERO;
        for i in 0..num_vertices {
            Cartesian3::from_array(&extrude_directions, Some(i * 3), &mut to_encode1);
            if Cartesian3::equals(Some(&to_encode1), Some(&Cartesian3::ZERO)) {
                i2 += 2;
                continue;
            }
            AttributeCompression::oct_encode_in_range(&to_encode1, 65535.0, &mut encode_result2);
            compressed_directions[i2] = encode_result2.x;
            compressed_directions[i2 + 1] = encode_result2.y;
            i2 += 2;
        }

        geometry.attributes.insert(
            "compressedAttributes".to_string(),
            GeometryAttribute::new(
                ComponentDatatype::Float,
                2,
                false,
                compressed_directions,
            ),
        );
        geometry.attributes.remove("extrudeDirection");
        return;
    }

    let normal_attribute = geometry.attributes.get("normal").map(|a| a.values.clone());
    let st_attribute = geometry.attributes.get("st").map(|a| a.values.clone());

    let has_normal = normal_attribute.is_some();
    let has_st = st_attribute.is_some();
    if !has_normal && !has_st {
        return;
    }

    let tangent_attribute = geometry.attributes.get("tangent").map(|a| a.values.clone());
    let bitangent_attribute = geometry.attributes.get("bitangent").map(|a| a.values.clone());

    let has_tangent = tangent_attribute.is_some();
    let has_bitangent = bitangent_attribute.is_some();

    let normals = normal_attribute.unwrap_or_default();
    let st = st_attribute.unwrap_or_default();
    let tangents = tangent_attribute.unwrap_or_default();
    let bitangents = bitangent_attribute.unwrap_or_default();

    let length = if has_normal { normals.len() } else { st.len() };
    let num_components = if has_normal { 3.0 } else { 2.0 };
    let num_vertices = (length as f64 / num_components) as usize;

    let mut compressed_length = num_vertices;
    let mut num_compressed_components = if has_st && has_normal { 2.0 } else { 1.0 };
    num_compressed_components += if has_tangent || has_bitangent { 1.0 } else { 0.0 };
    compressed_length = (compressed_length as f64 * num_compressed_components) as usize;

    let mut compressed_attributes = vec![0.0f64; compressed_length];

    let mut normal_index = 0usize;
    let mut scratch_cartesian2 = Cartesian2::ZERO;
    let mut to_encode1 = Cartesian3::ZERO;
    let mut to_encode2 = Cartesian3::ZERO;
    let mut to_encode3 = Cartesian3::ZERO;
    for i in 0..num_vertices {
        if has_st {
            Cartesian2::from_array(&st, Some(i * 2), &mut scratch_cartesian2);
            compressed_attributes[normal_index] =
                AttributeCompression::compress_texture_coordinates(&scratch_cartesian2);
            normal_index += 1;
        }

        let index = i * 3;
        if has_normal && has_tangent && has_bitangent {
            Cartesian3::from_array(&normals, Some(index), &mut to_encode1);
            Cartesian3::from_array(&tangents, Some(index), &mut to_encode2);
            Cartesian3::from_array(&bitangents, Some(index), &mut to_encode3);

            AttributeCompression::oct_pack(
                &to_encode1,
                &to_encode2,
                &to_encode3,
                &mut scratch_cartesian2,
            );
            compressed_attributes[normal_index] = scratch_cartesian2.x;
            compressed_attributes[normal_index + 1] = scratch_cartesian2.y;
            normal_index += 2;
        } else {
            if has_normal {
                Cartesian3::from_array(&normals, Some(index), &mut to_encode1);
                compressed_attributes[normal_index] =
                    AttributeCompression::oct_encode_float(&to_encode1);
                normal_index += 1;
            }

            if has_tangent {
                Cartesian3::from_array(&tangents, Some(index), &mut to_encode1);
                compressed_attributes[normal_index] =
                    AttributeCompression::oct_encode_float(&to_encode1);
                normal_index += 1;
            }

            if has_bitangent {
                Cartesian3::from_array(&bitangents, Some(index), &mut to_encode1);
                compressed_attributes[normal_index] =
                    AttributeCompression::oct_encode_float(&to_encode1);
                normal_index += 1;
            }
        }
    }

    geometry.attributes.insert(
        "compressedAttributes".to_string(),
        GeometryAttribute::new(
            ComponentDatatype::Float,
            num_compressed_components as u32,
            false,
            compressed_attributes,
        ),
    );

    if has_normal {
        geometry.attributes.remove("normal");
    }
    if has_st {
        geometry.attributes.remove("st");
    }
    if has_bitangent {
        geometry.attributes.remove("bitangent");
    }
    if has_tangent {
        geometry.attributes.remove("tangent");
    }
}
