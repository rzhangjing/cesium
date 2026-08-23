//! Ported from `packages/engine/Source/Scene/decodeMvt.js`.

/// Decodes MVT data.
pub struct DecodeMvt {
    _private: (),
}

impl DecodeMvt {
    /// Creates a new DecodeMvt.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for DecodeMvt {
    fn default() -> Self { Self::new() }
}
