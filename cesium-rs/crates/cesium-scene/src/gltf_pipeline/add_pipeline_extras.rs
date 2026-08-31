//! Ported from `packages/engine/Source/Scene/GltfPipeline/addPipelineExtras.js`.

use serde_json::Value;

use crate::gltf_pipeline::defined;
use crate::gltf_pipeline::for_each;

/// Adds extras._pipeline to each object that can have extras in the glTF
/// asset. This stage runs before updateVersion and handles both glTF 1.0
/// and glTF 2.0 assets.
///
/// DEVIATION: the JS also prepares `extras._pipeline.source` slots on
/// buffers; the Rust port keeps binary sources in the parallel side table
/// (see `gltf_pipeline` module docs), so this only ensures the
/// `extras._pipeline` JSON object exists.
pub fn add_pipeline_extras(gltf: &mut Value) -> &mut Value {
    for_each::shader(gltf, |shader, _| {
        add_extras(shader);
        None::<()>
    });
    for_each::buffer(gltf, |buffer, _| {
        add_extras(buffer);
        None::<()>
    });
    for_each::image(gltf, |image, _| {
        add_extras(image);
        None::<()>
    });

    add_extras(gltf);
    gltf
}

/// Ensures `object.extras._pipeline` exists (mirrors JS `addExtras`).
pub(crate) fn add_extras(object: &mut Value) {
    if !defined(object.get("extras")) {
        object["extras"] = Value::Object(serde_json::Map::new());
    }
    if !defined(object["extras"].get("_pipeline")) {
        object["extras"]["_pipeline"] = Value::Object(serde_json::Map::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn add_pipeline_extras_adds_extras_to_buffers_images_and_root() {
        let mut gltf = json!({
            "buffers": [{ "byteLength": 4 }],
            "images": [{ "uri": "a.png" }]
        });
        add_pipeline_extras(&mut gltf);

        assert_eq!(gltf["buffers"][0]["extras"]["_pipeline"], json!({}));
        assert_eq!(gltf["images"][0]["extras"]["_pipeline"], json!({}));
        assert_eq!(gltf["extras"]["_pipeline"], json!({}));
    }

    #[test]
    fn add_pipeline_extras_preserves_existing_extras() {
        let mut gltf = json!({
            "buffers": [{ "byteLength": 4, "extras": { "custom": 1 } }]
        });
        add_pipeline_extras(&mut gltf);

        assert_eq!(gltf["buffers"][0]["extras"]["custom"], 1);
        assert_eq!(gltf["buffers"][0]["extras"]["_pipeline"], json!({}));
    }

    #[test]
    fn add_pipeline_extras_handles_khr_techniques_shaders() {
        let mut gltf = json!({
            "extensionsUsed": ["KHR_techniques_webgl"],
            "extensions": {
                "KHR_techniques_webgl": { "shaders": [{ "type": 35633 }] }
            }
        });
        add_pipeline_extras(&mut gltf);

        let shader = &gltf["extensions"]["KHR_techniques_webgl"]["shaders"][0];
        assert_eq!(shader["extras"]["_pipeline"], json!({}));
    }
}
