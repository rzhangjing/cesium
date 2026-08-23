//! Ported from `packages/engine/Source/Scene/B3dmParser.js`.

/// Parses batched 3D model (b3dm) tiles.
pub struct B3dmParser {
    _private: (),
}

impl B3dmParser {
    /// Creates a new B3dmParser.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for B3dmParser {
    fn default() -> Self { Self::new() }
}
