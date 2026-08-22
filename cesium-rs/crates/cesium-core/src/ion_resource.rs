//! Ported from `packages/engine/Source/Core/IonResource.js`.
//!
//! Cesium Ion resource management.

/// A resource on the Cesium Ion platform.
/// Skeleton: requires network I/O.
pub struct IonResource {
    _url: String,
}

impl IonResource {
    /// Creates a new Ion resource.
    pub fn new(url: String) -> Self {
        Self { _url: url }
    }

    /// Returns the URL.
    pub fn url(&self) -> &str {
        &self._url
    }
}
