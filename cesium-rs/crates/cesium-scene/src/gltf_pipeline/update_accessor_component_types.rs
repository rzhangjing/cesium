//! Ported from
//! `packages/engine/Source/Scene/GltfPipeline/updateAccessorComponentTypes.js`.

use cesium_core::runtime_error::RuntimeError;
use cesium_core::webgl_constants::WebGLConstants;
use serde_json::{json, Value};

use crate::gltf_pipeline::add_buffer::add_buffer;
use crate::gltf_pipeline::for_each;
use crate::gltf_pipeline::read_accessor_packed::read_accessor_packed;
use crate::gltf_pipeline::PipelineBufferSources;

/// Updates accessors referenced by `JOINTS_0` and `WEIGHTS_0` attributes to
/// use correct component types.
///
/// # Errors
/// Propagates [`RuntimeError`] from `readAccessorPacked` when an accessor's
/// buffer has no attached binary source.
pub fn update_accessor_component_types(
    gltf: &mut Value,
    sources: &mut PipelineBufferSources,
) -> Result<(), RuntimeError> {
    // The ids are collected up front so the borrow on gltf is released
    // before convert_type reads/writes it.
    let joints_ids = for_each::accessor_ids_with_semantic(gltf, "JOINTS_0");
    for accessor_id in joints_ids {
        let accessor = gltf["accessors"][accessor_id].clone();
        let component_type = accessor
            .get("componentType")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as u32;
        if component_type == WebGLConstants::BYTE {
            convert_type(gltf, sources, accessor_id, WebGLConstants::UNSIGNED_BYTE)?;
        } else if component_type != WebGLConstants::UNSIGNED_BYTE
            && component_type != WebGLConstants::UNSIGNED_SHORT
        {
            convert_type(gltf, sources, accessor_id, WebGLConstants::UNSIGNED_SHORT)?;
        }
    }

    let weights_ids = for_each::accessor_ids_with_semantic(gltf, "WEIGHTS_0");
    for accessor_id in weights_ids {
        let accessor = gltf["accessors"][accessor_id].clone();
        let component_type = accessor
            .get("componentType")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as u32;
        if component_type == WebGLConstants::BYTE {
            convert_type(gltf, sources, accessor_id, WebGLConstants::UNSIGNED_BYTE)?;
        } else if component_type == WebGLConstants::SHORT {
            convert_type(gltf, sources, accessor_id, WebGLConstants::UNSIGNED_SHORT)?;
        }
    }

    Ok(())
}

fn convert_type(
    gltf: &mut Value,
    sources: &mut PipelineBufferSources,
    accessor_id: usize,
    updated_component_type: u32,
) -> Result<(), RuntimeError> {
    let accessor = gltf["accessors"][accessor_id].clone();
    let packed = read_accessor_packed(gltf, &accessor, sources)?;
    // ComponentDatatype.createTypedArray(updatedComponentType, packed):
    // JS typed-array construction applies ToUint8 / ToUint16 (truncating
    // fractional parts and wrapping modulo 2^bits).
    let new_buffer: Vec<u8> = match updated_component_type {
        WebGLConstants::UNSIGNED_BYTE => packed
            .iter()
            .map(|value| value.trunc().rem_euclid(256.0) as u8)
            .collect(),
        WebGLConstants::UNSIGNED_SHORT => packed
            .iter()
            .flat_map(|value| ((value.trunc().rem_euclid(65536.0)) as u16).to_le_bytes())
            .collect(),
        other => unreachable!("unsupported updated componentType {other}"),
    };
    let buffer_view = add_buffer(gltf, sources, new_buffer);
    gltf["accessors"][accessor_id]["bufferView"] = json!(buffer_view);
    gltf["accessors"][accessor_id]["componentType"] = json!(updated_component_type);
    gltf["accessors"][accessor_id]["byteOffset"] = json!(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gltf_pipeline::PipelineBufferSource;

    fn joints_gltf(component_type: u32, bytes: Vec<u8>) -> (Value, PipelineBufferSources) {
        let gltf = json!({
            "accessors": [
                {
                    "bufferView": 0,
                    "byteOffset": 0,
                    "componentType": component_type,
                    "count": 2,
                    "type": "VEC4"
                }
            ],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0 }],
            "buffers": [{ "byteLength": 0 }],
            "meshes": [
                { "primitives": [{ "attributes": { "JOINTS_0": 0 } }] }
            ]
        });
        let sources = vec![Some(PipelineBufferSource::new(bytes))];
        (gltf, sources)
    }

    #[test]
    fn byte_joints_become_unsigned_byte() {
        let (mut gltf, mut sources) = joints_gltf(WebGLConstants::BYTE, vec![0, 1, 2, 3, 4, 5, 6, 7]);
        update_accessor_component_types(&mut gltf, &mut sources).unwrap();
        assert_eq!(gltf["accessors"][0]["componentType"], json!(WebGLConstants::UNSIGNED_BYTE));
        assert_eq!(gltf["accessors"][0]["byteOffset"], json!(0));
        // A new buffer/bufferView pair was appended for the converted data.
        assert_eq!(gltf["buffers"].as_array().unwrap().len(), 2);
        let new_view = gltf["accessors"][0]["bufferView"].as_u64().unwrap() as usize;
        assert_eq!(new_view, 1);
        let converted = sources[1].as_ref().unwrap().as_bytes().to_vec();
        assert_eq!(converted, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn short_joints_become_unsigned_short() {
        // count=2 VEC4 accessors need 8 components of source data.
        let bytes: Vec<u8> = [-1i16, 5, 300, 0, -1, 5, 300, 0]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        let (mut gltf, mut sources) = joints_gltf(WebGLConstants::SHORT, bytes);
        update_accessor_component_types(&mut gltf, &mut sources).unwrap();
        assert_eq!(
            gltf["accessors"][0]["componentType"],
            json!(WebGLConstants::UNSIGNED_SHORT)
        );
        let converted = sources[1].as_ref().unwrap().as_bytes().to_vec();
        let expected: Vec<u16> = vec![65535, 5, 300, 0, 65535, 5, 300, 0];
        assert_eq!(
            converted,
            expected.iter().flat_map(|value| value.to_le_bytes()).collect::<Vec<u8>>()
        );
    }

    #[test]
    fn unsigned_short_joints_are_untouched() {
        let bytes: Vec<u8> = [0u16, 1, 2, 3]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        let (mut gltf, mut sources) = joints_gltf(WebGLConstants::UNSIGNED_SHORT, bytes);
        update_accessor_component_types(&mut gltf, &mut sources).unwrap();
        assert_eq!(
            gltf["accessors"][0]["componentType"],
            json!(WebGLConstants::UNSIGNED_SHORT)
        );
        assert_eq!(gltf["buffers"].as_array().unwrap().len(), 1);
    }
}
