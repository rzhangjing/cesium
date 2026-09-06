//! Ported from `packages/engine/Source/Scene/Model/B3dmLoader.js`.

/// B3DM loader.
///
/// Loads Batched 3D Model content from binary data.
pub struct B3dmLoader {
    /// Whether loading is complete.
    pub complete: bool,
}

impl B3dmLoader {
    /// Creates a new B3dmLoader.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for B3dmLoader {
    fn default() -> Self { Self::new() }
}
