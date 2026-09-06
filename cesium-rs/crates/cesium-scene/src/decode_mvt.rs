//! Ported from `packages/engine/Source/Scene/DecodeMvt.js`.

/// MVT decoder.
///
/// Decodes Mapbox Vector Tile binary data.
pub struct DecodeMvt {
    /// Whether decoding is complete.
    pub complete: bool,
}

impl DecodeMvt {
    /// Creates a new DecodeMvt.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for DecodeMvt {
    fn default() -> Self { Self::new() }
}
