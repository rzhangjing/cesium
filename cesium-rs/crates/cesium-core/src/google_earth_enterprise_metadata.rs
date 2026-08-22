//! Ported from `packages/engine/Source/Core/GoogleEarthEnterpriseMetadata.js`.
//!
//! Metadata for Google Earth Enterprise terrain tiles.

/// Metadata for Google Earth Enterprise terrain.
/// Skeleton: requires network I/O and binary parsing.
pub struct GoogleEarthEnterpriseMetadata {
    _url: String,
}

impl GoogleEarthEnterpriseMetadata {
    /// Creates new metadata from options.
    pub fn new(url: String) -> Self {
        Self { _url: url }
    }

    /// Returns the URL.
    pub fn url(&self) -> &str {
        &self._url
    }
}
