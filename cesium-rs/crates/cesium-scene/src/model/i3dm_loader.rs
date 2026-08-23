//! Ported from `packages/engine/Source/Scene/I3dmLoader.js`.
//!
//! Loads instanced 3D model tiles.

/// Loads instanced 3D model (i3dm) tiles.
pub struct I3dmLoader;

impl I3dmLoader {
    pub fn load(_data: &[u8]) -> Option<Vec<u8>> {
        None
    }
}
