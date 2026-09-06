//! Ported from `packages/engine/Source/Scene/I3SLayer.js`.

/// An I3S layer.
///
/// Represents a single layer within an I3S service.
pub struct I3SLayer {
    /// The layer name.
    pub name: String,
    /// The layer URL.
    pub url: String,
    /// Whether the layer is loaded.
    pub loaded: bool,
}

impl I3SLayer {
    /// Creates a new I3SLayer.
    pub fn new() -> Self { Self { name: String::new(), url: String::new(), loaded: false } }
}

impl Default for I3SLayer {
    fn default() -> Self { Self::new() }
}
