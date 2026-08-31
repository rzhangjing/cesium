//! Ported from `packages/engine/Source/Scene/GltfPipeline/removeExtension.js`.

use serde_json::{json, Value};

use crate::gltf_pipeline::for_each;
use crate::gltf_pipeline::remove_extensions_used::remove_extensions_used;

/// Removes an extension from gltf.extensions, gltf.extensionsUsed,
/// gltf.extensionsRequired, and any other objects in the glTF if it is
/// present.
///
/// Returns the extension data removed from `gltf.extensions` (the JS
/// return value of `removeExtensionAndTraverse` on the top-level object).
pub fn remove_extension(gltf: &mut Value, extension: &str) -> Option<Value> {
    // Also removes from extensionsRequired.
    remove_extensions_used(gltf, extension);

    if extension == "CESIUM_RTC" {
        remove_cesium_rtc(gltf);
    }

    remove_extension_and_traverse(gltf, extension)
}

/// Rewrites `CESIUM_RTC_MODELVIEW` uniform semantics to `MODELVIEW`
/// (mirrors JS `removeCesiumRTC`).
fn remove_cesium_rtc(gltf: &mut Value) {
    for_each::technique(gltf, |technique, _| {
        for_each::technique_uniform(technique, |uniform, _| {
            if uniform.get("semantic").and_then(|s| s.as_str()) == Some("CESIUM_RTC_MODELVIEW") {
                uniform["semantic"] = json!("MODELVIEW");
            }
            None::<()>
        });
        None::<()>
    });
}

/// Recursively removes `object.extensions[extension]` from every plain
/// object in the tree, deleting empty `extensions` containers. Returns the
/// removed data for the object the recursion was invoked on.
fn remove_extension_and_traverse(object: &mut Value, extension: &str) -> Option<Value> {
    match object {
        Value::Array(items) => {
            for item in items.iter_mut() {
                remove_extension_and_traverse(item, extension);
            }
            None
        }
        Value::Object(map) => {
            let mut extension_data = None;
            let mut extensions_now_empty = false;
            if let Some(extensions) = map.get_mut("extensions").and_then(|e| e.as_object_mut()) {
                if let Some(data) = extensions.remove(extension) {
                    if !data.is_null() {
                        extension_data = Some(data);
                    }
                    extensions_now_empty = extensions.is_empty();
                }
            }
            if extensions_now_empty {
                map.remove("extensions");
            }

            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                if let Some(child) = map.get_mut(&key) {
                    remove_extension_and_traverse(child, extension);
                }
            }
            extension_data
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_extension_returns_top_level_data_and_traverses() {
        let mut gltf = json!({
            "extensionsUsed": ["CESIUM_RTC"],
            "extensions": { "CESIUM_RTC": { "center": [1.0, 2.0, 3.0] } },
            "materials": [{
                "extensions": { "CESIUM_RTC": { "nested": true } }
            }]
        });

        let removed = remove_extension(&mut gltf, "CESIUM_RTC");

        assert_eq!(removed, Some(json!({ "center": [1.0, 2.0, 3.0] })));
        assert!(gltf.get("extensions").is_none());
        assert!(gltf["materials"][0].get("extensions").is_none());
        assert!(gltf.get("extensionsUsed").is_none());
    }

    #[test]
    fn remove_extension_rewrites_cesium_rtc_modelview_semantics() {
        let mut gltf = json!({
            "extensionsUsed": ["CESIUM_RTC"],
            "techniques": [{
                "uniforms": {
                    "u_modelViewMatrix": { "semantic": "CESIUM_RTC_MODELVIEW" }
                }
            }]
        });

        remove_extension(&mut gltf, "CESIUM_RTC");

        assert_eq!(
            gltf["techniques"][0]["uniforms"]["u_modelViewMatrix"]["semantic"],
            "MODELVIEW"
        );
    }

    #[test]
    fn remove_extension_missing_returns_none() {
        let mut gltf = json!({});
        assert_eq!(remove_extension(&mut gltf, "KHR_blend"), None);
    }
}
