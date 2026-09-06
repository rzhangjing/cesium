//! Ported from `packages/engine/Source/Scene/I3SDecoder.js`.

/// I3S data decoder.
///
/// Decodes compressed I3S geometry and attribute data.
pub struct I3SDecoder {
    /// Whether the decoder is ready.
    pub ready: bool,
}

impl I3SDecoder {
    /// Creates a new I3SDecoder.
    pub fn new() -> Self { Self { ready: false } }
}

impl Default for I3SDecoder {
    fn default() -> Self { Self::new() }
}
