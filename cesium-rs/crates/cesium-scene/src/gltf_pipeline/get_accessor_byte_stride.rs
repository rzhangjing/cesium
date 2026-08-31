//! Ported from `packages/engine/Source/Scene/GltfPipeline/getAccessorByteStride.js`.

use cesium_core::component_datatype::ComponentDatatype;
use serde_json::Value;

use crate::gltf_pipeline::defined;
use crate::gltf_pipeline::number_of_components_for_type::number_of_components_for_type;

/// Returns the byte stride of the provided accessor.
/// If the byteStride is 0, it is calculated based on type and componentType.
///
/// # Panics
/// Panics in debug builds when the accessor's `componentType` or `type` is
/// not a valid glTF value (the JS returns `NaN`-propagating values).
pub fn get_accessor_byte_stride(gltf: &Value, accessor: &Value) -> usize {
    let buffer_view_id = accessor.get("bufferView");
    if defined(buffer_view_id) {
        let index = buffer_view_id.and_then(|value| value.as_u64()).unwrap_or(0) as usize;
        if let Some(buffer_view) = gltf.get("bufferViews").and_then(|views| views.get(index)) {
            let byte_stride = buffer_view.get("byteStride");
            if defined(byte_stride) && byte_stride.and_then(|value| value.as_u64()).unwrap_or(0) > 0
            {
                return byte_stride.and_then(|value| value.as_u64()).unwrap_or(0) as usize;
            }
        }
    }

    let component_type = accessor
        .get("componentType")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as u32;
    let size_in_bytes = ComponentDatatype::try_from_u32(component_type)
        .expect("accessor.componentType is not a valid ComponentDatatype")
        .size_in_bytes();
    let accessor_type = accessor
        .get("type")
        .and_then(|value| value.as_str())
        .expect("accessor.type is required");
    size_in_bytes * number_of_components_for_type(accessor_type)
        .expect("accessor.type is not a valid glTF type")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn byte_stride_uses_buffer_view_stride_when_positive() {
        let gltf = json!({
            "bufferViews": [{ "byteStride": 24 }]
        });
        let accessor = json!({
            "bufferView": 0,
            "componentType": 5126,
            "type": "VEC3"
        });
        assert_eq!(get_accessor_byte_stride(&gltf, &accessor), 24);
    }

    #[test]
    fn byte_stride_computed_from_component_type_and_type() {
        let gltf = json!({
            "bufferViews": [{ "byteStride": 0 }]
        });
        let accessor = json!({
            "bufferView": 0,
            "componentType": 5126,
            "type": "VEC3"
        });
        assert_eq!(get_accessor_byte_stride(&gltf, &accessor), 12);
    }

    #[test]
    fn byte_stride_without_buffer_view() {
        let gltf = json!({});
        let accessor = json!({ "componentType": 5123, "type": "SCALAR" });
        assert_eq!(get_accessor_byte_stride(&gltf, &accessor), 2);
    }
}
