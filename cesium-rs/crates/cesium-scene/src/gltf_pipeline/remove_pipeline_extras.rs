//! Ported from `packages/engine/Source/Scene/GltfPipeline/removePipelineExtras.js`.

use serde_json::Value;

use crate::gltf_pipeline::defined;
use crate::gltf_pipeline::for_each;

/// Iterate through the objects within the glTF and delete their pipeline
/// extras object.
///
/// DEVIATION: the binary payloads carried by JS `extras._pipeline.source`
/// live in the parallel sources side table in the Rust port (see
/// `gltf_pipeline` module docs); this function only strips the JSON.
pub fn remove_pipeline_extras(gltf: &mut Value) -> &mut Value {
    for_each::shader(gltf, |shader, _| {
        remove_extras(shader);
        None::<()>
    });
    for_each::buffer(gltf, |buffer, _| {
        remove_extras(buffer);
        None::<()>
    });
    for_each::image(gltf, |image, _| {
        remove_extras(image);
        None::<()>
    });

    remove_extras(gltf);
    gltf
}

/// Removes `object.extras._pipeline`, deleting `extras` when it becomes
/// empty (mirrors JS `removeExtras`).
pub(crate) fn remove_extras(object: &mut Value) {
    if !defined(object.get("extras")) {
        return;
    }

    let extras_empty = if let Some(extras) = object.get_mut("extras") {
        if let Some(extras_object) = extras.as_object_mut() {
            extras_object.remove("_pipeline");
            extras_object.is_empty()
        } else {
            false
        }
    } else {
        false
    };

    if extras_empty {
        if let Some(object) = object.as_object_mut() {
            object.remove("extras");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn remove_pipeline_extras_strips_pipeline_and_empty_extras() {
        let mut gltf = json!({
            "buffers": [{ "byteLength": 4, "extras": { "_pipeline": { "source": [] } } }],
            "extras": { "_pipeline": {} }
        });
        remove_pipeline_extras(&mut gltf);

        assert!(gltf["buffers"][0].get("extras").is_none());
        assert!(gltf.get("extras").is_none());
    }

    #[test]
    fn remove_pipeline_extras_keeps_other_extras() {
        let mut gltf = json!({
            "images": [{ "uri": "a.png", "extras": { "_pipeline": {}, "name": "a" } }]
        });
        remove_pipeline_extras(&mut gltf);

        assert_eq!(gltf["images"][0]["extras"], json!({ "name": "a" }));
    }
}
