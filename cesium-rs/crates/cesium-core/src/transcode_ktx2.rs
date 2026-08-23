//! Ported from `packages/engine/Source/Core/transcodeKTX2.js`.

/// Transcodes KTX2 data.
pub struct TranscodeKtx2 {
    _private: (),
}

impl TranscodeKtx2 {
    /// Creates a new TranscodeKtx2.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TranscodeKtx2 {
    fn default() -> Self { Self::new() }
}
