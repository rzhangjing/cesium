//! Ported from `packages/engine/Source/Scene/findMeshoptExtension.js`.

use serde_json::Value;

/// Returns the meshopt compression extension object,
/// `KHR_meshopt_compression` or `EXT_meshopt_compression`, on a glTF
/// bufferView or buffer. If both are present, KHR is preferred.
///
/// Mirrors `findMeshoptExtension(gltfObject)`.
pub fn find_meshopt_extension(gltf_object: &Value) -> Option<&Value> {
    let extensions = gltf_object.get("extensions")?;
    extensions
        .get("KHR_meshopt_compression")
        .or_else(|| extensions.get("EXT_meshopt_compression"))
}
