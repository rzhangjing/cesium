//! Ported from `packages/engine/Source/Scene/PntsParser.js`.

/// PNTS parser.
///
/// Parses Point Cloud (.pnts) tile content.
pub struct PntsParser {
    /// Whether parsing is complete.
    pub complete: bool,
}

impl PntsParser {
    /// Creates a new PntsParser.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for PntsParser {
    fn default() -> Self { Self::new() }
}
