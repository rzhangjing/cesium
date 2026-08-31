//! Ported from `packages/engine/Source/Scene/GltfPipeline/readAccessorPacked.js`.

use cesium_core::component_datatype::ComponentDatatype;
use cesium_core::runtime_error::RuntimeError;
use serde_json::Value;

use crate::gltf_pipeline::defined;
use crate::gltf_pipeline::get_accessor_byte_stride::get_accessor_byte_stride;
use crate::gltf_pipeline::get_component_reader::get_component_reader;
use crate::gltf_pipeline::number_of_components_for_type::number_of_components_for_type;
use crate::gltf_pipeline::PipelineBufferSources;

/// Returns the accessor data in a contiguous array.
///
/// # Errors
/// Returns a [`RuntimeError`] when the accessor's buffer has no attached
/// binary source (the Rust analogue of `buffer.extras._pipeline.source`
/// being absent, which throws in JS).
pub fn read_accessor_packed(
    gltf: &Value,
    accessor: &Value,
    sources: &PipelineBufferSources,
) -> Result<Vec<f64>, RuntimeError> {
    let byte_stride = get_accessor_byte_stride(gltf, accessor);
    let component_type = accessor
        .get("componentType")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as u32;
    let component_type_byte_length = ComponentDatatype::try_from_u32(component_type)
        .expect("accessor.componentType is not a valid ComponentDatatype")
        .size_in_bytes();
    let accessor_type = accessor
        .get("type")
        .and_then(|value| value.as_str())
        .expect("accessor.type is required");
    let number_of_components = number_of_components_for_type(accessor_type)
        .expect("accessor.type is not a valid glTF type");
    let count = accessor
        .get("count")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize;
    let mut values = vec![0.0f64; number_of_components * count];

    let buffer_view_id = accessor.get("bufferView");
    if !defined(buffer_view_id) {
        return Ok(values);
    }

    let buffer_view_id = buffer_view_id.and_then(|value| value.as_u64()).unwrap_or(0) as usize;
    let buffer_view = gltf
        .get("bufferViews")
        .and_then(|views| views.get(buffer_view_id))
        .ok_or_else(|| RuntimeError::new(Some("Invalid bufferView id")))?;
    let buffer_id = buffer_view
        .get("buffer")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize;
    let source = sources
        .get(buffer_id)
        .and_then(|source| source.as_ref())
        .ok_or_else(|| {
            RuntimeError::new(Some("buffer has no attached pipeline binary source"))
        })?;

    let mut byte_offset = accessor
        .get("byteOffset")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize
        + buffer_view
            .get("byteOffset")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as usize
        + source.byte_offset;
    let component_reader = get_component_reader(component_type);
    let mut components = vec![0.0f64; number_of_components];

    for i in 0..count {
        component_reader.read(
            &source.buffer,
            byte_offset,
            number_of_components,
            component_type_byte_length,
            &mut components,
        );
        for j in 0..number_of_components {
            values[i * number_of_components + j] = components[j];
        }
        byte_offset += byte_stride;
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gltf_pipeline::PipelineBufferSource;
    use serde_json::json;

    #[test]
    fn read_accessor_packed_reads_interleaved_elements() {
        // VEC2 unsigned shorts with byteStride 8: (1,2) pad (3,4) pad
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 4]);
        bytes.extend_from_slice(&3u16.to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 4]);

        let gltf = json!({
            "buffers": [{ "byteLength": 16 }],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteStride": 8 }],
            "accessors": [{
                "bufferView": 0,
                "byteOffset": 0,
                "componentType": 5123,
                "count": 2,
                "type": "VEC2"
            }]
        });
        let sources = vec![Some(PipelineBufferSource::new(bytes))];

        let values = read_accessor_packed(&gltf, &gltf["accessors"][0], &sources).unwrap();
        assert_eq!(values, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn read_accessor_packed_without_buffer_view_is_zeros() {
        let gltf = json!({});
        let accessor = json!({ "componentType": 5126, "count": 2, "type": "SCALAR" });
        let sources: PipelineBufferSources = Vec::new();
        let values = read_accessor_packed(&gltf, &accessor, &sources).unwrap();
        assert_eq!(values, vec![0.0, 0.0]);
    }

    #[test]
    fn read_accessor_packed_respects_byte_offsets() {
        // bufferView.byteOffset 4 + accessor.byteOffset 4 → read at byte 8.
        let mut bytes = vec![0u8; 8];
        bytes.extend_from_slice(&7u16.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 2]);

        let gltf = json!({
            "buffers": [{ "byteLength": 12 }],
            "bufferViews": [{ "buffer": 0, "byteOffset": 4 }],
            "accessors": [{
                "bufferView": 0,
                "byteOffset": 4,
                "componentType": 5123,
                "count": 1,
                "type": "SCALAR"
            }]
        });
        let sources = vec![Some(PipelineBufferSource::new(bytes))];

        let values = read_accessor_packed(&gltf, &gltf["accessors"][0], &sources).unwrap();
        assert_eq!(values, vec![7.0]);
    }
}
