//! Ported from `packages/engine/Source/Scene/I3dmParser.js`.

/// I3DM parser.
pub struct I3dmParser {
    _private: (),
}

impl I3dmParser {
    /// Creates a new I3dmParser.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for I3dmParser {
    fn default() -> Self { Self::new() }
}
