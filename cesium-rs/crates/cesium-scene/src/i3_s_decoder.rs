//! Ported from `packages/engine/Source/Scene/I3SDecoder.js`.

/// I3S decoder.
pub struct I3SDecoder {
    _private: (),
}

impl I3SDecoder {
    /// Creates a new I3SDecoder.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for I3SDecoder {
    fn default() -> Self { Self::new() }
}
