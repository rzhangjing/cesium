//! Ported from `packages/engine/Source/Core/KTX2Transcoder.js`.

/// Transcodes KTX2 textures.
pub struct Ktx2Transcoder {
    _private: (),
}

impl Ktx2Transcoder {
    /// Creates a new Ktx2Transcoder.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Ktx2Transcoder {
    fn default() -> Self { Self::new() }
}
