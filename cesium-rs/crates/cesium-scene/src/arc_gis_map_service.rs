//! Ported from `packages/engine/Source/Scene/ArcGisMapService.js`.

/// An ArcGIS map service client.
pub struct ArcGisMapService {
    _private: (),
}

impl ArcGisMapService {
    /// Creates a new ArcGisMapService.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ArcGisMapService {
    fn default() -> Self { Self::new() }
}
