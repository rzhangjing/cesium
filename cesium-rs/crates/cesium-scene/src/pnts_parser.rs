//! Ported from `packages/engine/Source/Scene/PntsParser.js`.

/// Parses point cloud (pnts) tiles.
pub struct PntsParser {
    _private: (),
}

impl PntsParser {
    /// Creates a new PntsParser.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PntsParser {
    fn default() -> Self { Self::new() }
}
