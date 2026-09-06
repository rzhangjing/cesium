//! Ported from `packages/engine/Source/Scene/UrlTemplate3DTilesDataProvider.js`.

/// URL template 3D tiles data provider.
///
/// Provides 3D tile data from URL templates with variable substitution.
pub struct UrlTemplate3DTilesDataProvider {
    /// The URL template.
    pub url_template: String,
    /// Whether the provider is ready.
    pub ready: bool,
}

impl UrlTemplate3DTilesDataProvider {
    /// Creates a new UrlTemplate3DTilesDataProvider.
    pub fn new() -> Self { Self { url_template: String::new(), ready: false } }
}

impl Default for UrlTemplate3DTilesDataProvider {
    fn default() -> Self { Self::new() }
}
