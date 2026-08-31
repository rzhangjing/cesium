//! Ported from `packages/engine/Source/Scene/GltfPipeline/removeExtensionsRequired.js`.

use serde_json::Value;

/// Removes an extension from gltf.extensionsRequired if it is present.
pub fn remove_extensions_required(gltf: &mut Value, extension: &str) {
    let Some(object) = gltf.as_object_mut() else {
        return;
    };
    let mut remove_key = false;
    if let Some(Value::Array(extensions_required)) = object.get_mut("extensionsRequired") {
        extensions_required.retain(|item| item.as_str() != Some(extension));
        if extensions_required.is_empty() {
            remove_key = true;
        }
    }
    if remove_key {
        object.remove("extensionsRequired");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn remove_extensions_required_removes_and_deletes_empty() {
        let mut gltf = json!({ "extensionsRequired": ["KHR_blend", "CESIUM_RTC"] });
        remove_extensions_required(&mut gltf, "KHR_blend");
        assert_eq!(gltf["extensionsRequired"], json!(["CESIUM_RTC"]));
        remove_extensions_required(&mut gltf, "CESIUM_RTC");
        assert!(gltf.get("extensionsRequired").is_none());
    }

    #[test]
    fn remove_extensions_required_missing_is_noop() {
        let mut gltf = json!({});
        remove_extensions_required(&mut gltf, "KHR_blend");
        assert!(gltf.get("extensionsRequired").is_none());
    }
}
