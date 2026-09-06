//! Ported from `packages/engine/Source/Scene/hasExtension.js`.
//!
//! Checks whether a specific extension is present on a JSON object.
//! Works for both 3D Tiles extensions and glTF extensions.

use serde_json::Value;

/// Checks if a specific extension is present on a JSON object.
///
/// Mirrors CesiumJS `hasExtension(json, extensionName)`:
/// returns `true` when `json.extensions[extensionName]` is defined.
pub fn has_extension(json: &Value, extension_name: &str) -> bool {
    json.get("extensions")
        .and_then(|ext| ext.get(extension_name))
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn returns_true_when_extension_present() {
        let json = json!({
            "extensions": {
                "3DTILES_metadata": { "class": "buildings" }
            }
        });
        assert!(has_extension(&json, "3DTILES_metadata"));
    }

    #[test]
    fn returns_false_when_extension_absent() {
        let json = json!({ "extensions": {} });
        assert!(!has_extension(&json, "3DTILES_metadata"));
    }

    #[test]
    fn returns_false_when_no_extensions_key() {
        let json = json!({ "name": "test" });
        assert!(!has_extension(&json, "KHR_draco_mesh_compression"));
    }

    #[test]
    fn returns_false_for_null_json() {
        let json = Value::Null;
        assert!(!has_extension(&json, "anything"));
    }
}
