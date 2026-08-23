//! Ported from `packages/engine/Source/Scene/UrlTemplate3DTilesDataProvider.js`.

/// A URL template data provider for 3D Tiles.
pub struct UrlTemplate3DTilesDataProvider {
    _private: (),
}

impl UrlTemplate3DTilesDataProvider {
    /// Creates a new UrlTemplate3DTilesDataProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for UrlTemplate3DTilesDataProvider {
    fn default() -> Self { Self::new() }
}
