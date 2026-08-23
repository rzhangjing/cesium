//! Ported from `packages/engine/Source/Core/loadKTX2.js`.

/// Loads a KTX2 texture.
pub struct LoadKtx2 {
    _private: (),
}

impl LoadKtx2 {
    /// Creates a new LoadKtx2.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for LoadKtx2 {
    fn default() -> Self { Self::new() }
}
