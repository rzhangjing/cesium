//! Ported from `packages/engine/Source/Scene/GltfPipeline/usesExtension.js`.

use serde_json::Value;

use super::defined;

/// Checks whether the glTF uses the given extension.
pub fn uses_extension(gltf: &Value, extension: &str) -> bool {
    let extensions_used = gltf.get("extensionsUsed");
    defined(extensions_used)
        && extensions_used
            .and_then(|v| v.as_array())
            .is_some_and(|list| {
                list.iter()
                    .any(|item| item.as_str().is_some_and(|name| name == extension))
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn uses_extension_detects_listed_extension() {
        let gltf = json!({ "extensionsUsed": ["CESIUM_RTC"] });
        assert!(uses_extension(&gltf, "CESIUM_RTC"));
        assert!(!uses_extension(&gltf, "KHR_draco_mesh_compression"));
    }

    #[test]
    fn uses_extension_missing_list_is_false() {
        let gltf = json!({});
        assert!(!uses_extension(&gltf, "CESIUM_RTC"));
    }
}
