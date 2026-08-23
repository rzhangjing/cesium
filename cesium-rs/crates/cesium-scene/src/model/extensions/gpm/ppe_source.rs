//! Ported from `packages/engine/Source/Scene/Model/extensions/gpm/`.

/// Per-pixel effect source.
pub struct PpeSource {
    _private: (),
}

impl PpeSource {
    /// Creates a new PpeSource.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PpeSource {
    fn default() -> Self { Self::new() }
}
