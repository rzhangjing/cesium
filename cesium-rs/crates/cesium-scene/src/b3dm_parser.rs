//! Ported from `packages/engine/Source/Scene/B3dmParser.js`.

/// B3DM parser.
///
/// Parses Batched 3D Model (.b3dm) tile content.
pub struct B3dmParser {
    /// Whether parsing is complete.
    pub complete: bool,
}

impl B3dmParser {
    /// Creates a new B3dmParser.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for B3dmParser {
    fn default() -> Self { Self::new() }
}
