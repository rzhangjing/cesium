//! Ported from `packages/engine/Source/Scene/GltfPipeline/addBuffer.js`.

use serde_json::{json, Value};

use crate::gltf_pipeline::add_to_array::add_to_array_value;
use crate::gltf_pipeline::{PipelineBufferSource, PipelineBufferSources};

/// Adds buffer to gltf.
///
/// Returns the bufferView id of the newly added bufferView.
///
/// DEVIATION: the JS attaches the binary payload to
/// `buffer.extras._pipeline.source`; the Rust port stores it in the parallel
/// [`PipelineBufferSources`] side table instead (see module docs). The JSON
/// keeps `extras._pipeline` so `addPipelineExtras`/`removePipelineExtras`
/// invariants still hold.
pub fn add_buffer(gltf: &mut Value, sources: &mut PipelineBufferSources, buffer: Vec<u8>) -> usize {
    let byte_length = buffer.len();
    let new_buffer = json!({
        "byteLength": byte_length,
        "extras": {
            "_pipeline": {}
        }
    });
    let buffer_id = add_to_array_value(&mut gltf["buffers"], new_buffer, false);

    // Keep the binary side table aligned with `gltf["buffers"]`.
    while sources.len() <= buffer_id {
        sources.push(None);
    }
    sources[buffer_id] = Some(PipelineBufferSource::new(buffer));

    let buffer_view = json!({
        "buffer": buffer_id,
        "byteOffset": 0,
        "byteLength": byte_length
    });
    add_to_array_value(&mut gltf["bufferViews"], buffer_view, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_buffer_appends_buffer_view_and_source() {
        let mut gltf = json!({
            "buffers": [{ "byteLength": 4 }],
            "bufferViews": []
        });
        let mut sources: PipelineBufferSources = vec![None];

        let view_id = add_buffer(&mut gltf, &mut sources, vec![1, 2, 3, 4]);

        assert_eq!(view_id, 0);
        assert_eq!(gltf["buffers"][1]["byteLength"], 4);
        assert_eq!(gltf["bufferViews"][0]["buffer"], 1);
        assert_eq!(gltf["bufferViews"][0]["byteOffset"], 0);
        assert_eq!(gltf["bufferViews"][0]["byteLength"], 4);
        let source = sources[1].as_ref().expect("source attached");
        assert_eq!(source.as_bytes(), &[1, 2, 3, 4]);
        assert_eq!(source.byte_offset, 0);
    }

    #[test]
    fn add_buffer_creates_missing_arrays() {
        let mut gltf = json!({});
        let mut sources: PipelineBufferSources = Vec::new();

        let view_id = add_buffer(&mut gltf, &mut sources, vec![9]);

        assert_eq!(view_id, 0);
        assert_eq!(gltf["buffers"].as_array().unwrap().len(), 1);
        assert_eq!(sources.len(), 1);
    }
}
