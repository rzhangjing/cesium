//! Ported from `packages/engine/Source/Scene/Model/GeoJsonLoader.js`.

/// GeoJSON loader.
///
/// Loads GeoJSON data and converts to primitives.
pub struct GeoJsonLoader {
    /// Whether loading is complete.
    pub complete: bool,
}

impl GeoJsonLoader {
    /// Creates a new GeoJsonLoader.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for GeoJsonLoader {
    fn default() -> Self { Self::new() }
}
