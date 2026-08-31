//! Ported from `packages/engine/Source/Scene/GltfPipeline/removeExtensionsUsed.js`.

use serde_json::Value;

use crate::gltf_pipeline::remove_extensions_required::remove_extensions_required;

/// Removes an extension from gltf.extensionsUsed and gltf.extensionsRequired
/// if it is present.
pub fn remove_extensions_used(gltf: &mut Value, extension: &str) {
    let has_array = gltf
        .get("extensionsUsed")
        .is_some_and(|value| value.is_array());
    if !has_array {
        return;
    }

    let mut remove_key = false;
    if let Some(Value::Array(extensions_used)) = gltf.get_mut("extensionsUsed") {
        extensions_used.retain(|item| item.as_str() != Some(extension));
        if extensions_used.is_empty() {
            remove_key = true;
        }
    }
    remove_extensions_required(gltf, extension);
    if remove_key {
        if let Some(object) = gltf.as_object_mut() {
            object.remove("extensionsUsed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn remove_extensions_used_removes_used_and_required() {
        let mut gltf = json!({
            "extensionsUsed": ["CESIUM_RTC", "KHR_blend"],
            "extensionsRequired": ["CESIUM_RTC"]
        });
        remove_extensions_used(&mut gltf, "CESIUM_RTC");
        assert_eq!(gltf["extensionsUsed"], json!(["KHR_blend"]));
        assert!(gltf.get("extensionsRequired").is_none());
    }

    #[test]
    fn remove_extensions_used_deletes_empty_lists() {
        let mut gltf = json!({
            "extensionsUsed": ["CESIUM_RTC"],
            "extensionsRequired": ["CESIUM_RTC"]
        });
        remove_extensions_used(&mut gltf, "CESIUM_RTC");
        assert!(gltf.get("extensionsUsed").is_none());
        assert!(gltf.get("extensionsRequired").is_none());
    }
}
