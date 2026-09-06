//! Ported from `packages/engine/Source/Scene/BufferLoader.js`.

/// Buffer loader.
///
/// Loads vertex and index buffers from glTF data.
pub struct BufferLoader {
    /// Whether loading is complete.
    pub complete: bool,
}

impl BufferLoader {
    /// Creates a new BufferLoader.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for BufferLoader {
    fn default() -> Self { Self::new() }
}
