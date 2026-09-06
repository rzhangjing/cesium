//! Ported from `packages/engine/Source/Scene/I3SSublayer.js`.

/// An I3S sublayer.
///
/// Represents a sublayer within an I3S layer.
pub struct I3SSublayer {
    /// The sublayer index.
    pub index: u32,
    /// The sublayer name.
    pub name: String,
}

impl I3SSublayer {
    /// Creates a new I3SSublayer.
    pub fn new() -> Self { Self { index: 0, name: String::new() } }
}

impl Default for I3SSublayer {
    fn default() -> Self { Self::new() }
}
