//! Ported from `packages/engine/Source/Scene/GltfPipeline/addExtensionsRequired.js`.

use serde_json::{json, Value};

use crate::gltf_pipeline::add_extensions_used::add_extensions_used;
use crate::gltf_pipeline::add_to_array::add_to_array_value;

/// Adds an extension to gltf.extensionsRequired if it does not already exist.
/// Initializes extensionsRequired if it is not defined.
pub fn add_extensions_required(gltf: &mut Value, extension: &str) {
    add_to_array_value(&mut gltf["extensionsRequired"], json!(extension), true);
    add_extensions_used(gltf, extension);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_extensions_required_also_adds_used() {
        let mut gltf = json!({});
        add_extensions_required(&mut gltf, "KHR_techniques_webgl");
        add_extensions_required(&mut gltf, "KHR_techniques_webgl");
        assert_eq!(
            gltf["extensionsRequired"],
            json!(["KHR_techniques_webgl"])
        );
        assert_eq!(gltf["extensionsUsed"], json!(["KHR_techniques_webgl"]));
    }
}
