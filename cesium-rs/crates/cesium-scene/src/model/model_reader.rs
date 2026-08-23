//! Ported from `packages/engine/Source/Scene/ModelReader.js`.
//!
//! Reads glTF models.

/// Reads glTF models from JSON or binary (GLB) data.
pub struct ModelReader;

impl ModelReader {
    pub fn read_gltf(_json: &str) -> Option<Vec<u8>> {
        None
    }

    pub fn read_glb(_data: &[u8]) -> Option<Vec<u8>> {
        None
    }
}
