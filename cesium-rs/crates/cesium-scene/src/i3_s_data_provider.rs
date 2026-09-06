//! Ported from `packages/engine/Source/Scene/I3SDataProvider.js`.

/// I3S data provider.
///
/// Loads and manages OGC I3S (Indexed 3D Scene) layer data.
pub struct I3SDataProvider {
    /// The I3S service URL.
    pub url: String,
    /// Whether the provider is ready.
    pub ready: bool,
}

impl I3SDataProvider {
    /// Creates a new I3SDataProvider.
    pub fn new() -> Self { Self { url: String::new(), ready: false } }
}

impl Default for I3SDataProvider {
    fn default() -> Self { Self::new() }
}
