//! Ported from `packages/engine/Source/Scene/GltfPipeline/addExtensionsUsed.js`.

use serde_json::{json, Value};

use crate::gltf_pipeline::add_to_array::add_to_array_value;

/// Adds an extension to gltf.extensionsUsed if it does not already exist.
/// Initializes extensionsUsed if it is not defined.
pub fn add_extensions_used(gltf: &mut Value, extension: &str) {
    add_to_array_value(&mut gltf["extensionsUsed"], json!(extension), true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_extensions_used_initializes_and_dedupes() {
        let mut gltf = json!({});
        add_extensions_used(&mut gltf, "KHR_blend");
        add_extensions_used(&mut gltf, "KHR_blend");
        add_extensions_used(&mut gltf, "CESIUM_RTC");
        assert_eq!(gltf["extensionsUsed"], json!(["KHR_blend", "CESIUM_RTC"]));
    }
}
