//! Ported from `packages/engine/Source/Scene/Model/MappedPositions.js`.

/// Mapped positions for model rendering.
pub struct MappedPositions {
    _private: (),
}

impl MappedPositions {
    /// Creates a new MappedPositions.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MappedPositions {
    fn default() -> Self { Self::new() }
}
