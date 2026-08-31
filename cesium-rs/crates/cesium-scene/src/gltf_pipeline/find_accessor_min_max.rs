//! Ported from `packages/engine/Source/Scene/GltfPipeline/findAccessorMinMax.js`.

use cesium_core::component_datatype::ComponentDatatype;
use cesium_core::runtime_error::RuntimeError;
use serde_json::Value;

use crate::gltf_pipeline::defined;
use crate::gltf_pipeline::get_accessor_byte_stride::get_accessor_byte_stride;
use crate::gltf_pipeline::get_component_reader::get_component_reader;
use crate::gltf_pipeline::number_of_components_for_type::number_of_components_for_type;
use crate::gltf_pipeline::PipelineBufferSources;

/// The per-component min/max values of an accessor.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessorMinMax {
    /// Per-component minimum values.
    pub min: Vec<f64>,
    /// Per-component maximum values.
    pub max: Vec<f64>,
}

/// Finds the min and max values of the accessor.
///
/// # Errors
/// Returns a [`RuntimeError`] when the accessor's buffer has no attached
/// binary source (the Rust analogue of `buffer.extras._pipeline.source`
/// being absent, which throws in JS).
pub fn find_accessor_min_max(
    gltf: &Value,
    accessor: &Value,
    sources: &PipelineBufferSources,
) -> Result<AccessorMinMax, RuntimeError> {
    let accessor_type = accessor
        .get("type")
        .and_then(|value| value.as_str())
        .expect("accessor.type is required");
    let number_of_components = number_of_components_for_type(accessor_type)
        .expect("accessor.type is not a valid glTF type");

    // According to the spec, when bufferView is not defined, accessor must
    // be initialized with zeros.
    let buffer_view_id = accessor.get("bufferView");
    if !defined(buffer_view_id) {
        return Ok(AccessorMinMax {
            min: vec![0.0; number_of_components],
            max: vec![0.0; number_of_components],
        });
    }

    let mut min = vec![f64::INFINITY; number_of_components];
    let mut max = vec![f64::NEG_INFINITY; number_of_components];

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

    let count = accessor
        .get("count")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize;
    let byte_stride = get_accessor_byte_stride(gltf, accessor);
    let mut byte_offset = accessor
        .get("byteOffset")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize
        + buffer_view
            .get("byteOffset")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as usize
        + source.byte_offset;
    let component_type = accessor
        .get("componentType")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as u32;
    let component_type_byte_length = ComponentDatatype::try_from_u32(component_type)
        .expect("accessor.componentType is not a valid ComponentDatatype")
        .size_in_bytes();
    let component_reader = get_component_reader(component_type);
    let mut components = vec![0.0f64; number_of_components];

    for _ in 0..count {
        component_reader.read(
            &source.buffer,
            byte_offset,
            number_of_components,
            component_type_byte_length,
            &mut components,
        );
        for (j, value) in components.iter().enumerate() {
            if *value < min[j] {
                min[j] = *value;
            }
            if *value > max[j] {
                max[j] = *value;
            }
        }
        byte_offset += byte_stride;
    }

    Ok(AccessorMinMax { min, max })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gltf_pipeline::PipelineBufferSource;
    use serde_json::json;

    fn float_gltf() -> (Value, PipelineBufferSources) {
        // Two VEC2 float elements: (1.0, -2.0), (3.0, 4.0) with stride 16
        // (4 bytes of padding per element).
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1.0f32.to_le_bytes());
        bytes.extend_from_slice(&(-2.0f32).to_le_bytes());
        bytes.extend_from_slice(&[0u8; 8]);
        bytes.extend_from_slice(&3.0f32.to_le_bytes());
        bytes.extend_from_slice(&4.0f32.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 8]);

        let gltf = json!({
            "buffers": [{ "byteLength": 32 }],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteStride": 16 }],
            "accessors": [{
                "bufferView": 0,
                "byteOffset": 0,
                "componentType": 5126,
                "count": 2,
                "type": "VEC2"
            }]
        });
        let sources = vec![Some(PipelineBufferSource::new(bytes))];
        (gltf, sources)
    }

    #[test]
    fn find_accessor_min_max_computes_per_component_extrema() {
        let (gltf, sources) = float_gltf();
        let accessor = &gltf["accessors"][0];
        let min_max = find_accessor_min_max(&gltf, accessor, &sources).unwrap();
        assert_eq!(min_max.min, vec![1.0, -2.0]);
        assert_eq!(min_max.max, vec![3.0, 4.0]);
    }

    #[test]
    fn find_accessor_min_max_without_buffer_view_is_zeros() {
        let (gltf, sources) = float_gltf();
        let accessor = json!({ "componentType": 5126, "count": 3, "type": "VEC3" });
        let min_max = find_accessor_min_max(&gltf, &accessor, &sources).unwrap();
        assert_eq!(min_max.min, vec![0.0; 3]);
        assert_eq!(min_max.max, vec![0.0; 3]);
    }

    #[test]
    fn find_accessor_min_max_missing_source_errors() {
        let (gltf, sources) = float_gltf();
        let accessor = &gltf["accessors"][0];
        let empty: PipelineBufferSources = vec![None];
        assert!(find_accessor_min_max(&gltf, accessor, &empty).is_err());
        let _ = sources;
    }
}
