//! Ported from `packages/engine/Source/Scene/B3dmLoader.js`.

/// Loads B3DM files.
pub struct B3dmLoader {
    _private: (),
}

impl B3dmLoader {
    /// Creates a new B3dmLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for B3dmLoader {
    fn default() -> Self { Self::new() }
}
