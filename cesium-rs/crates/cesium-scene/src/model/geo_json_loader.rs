//! Ported from `packages/engine/Source/Scene/Model/GeoJsonLoader.js`.

/// Loads GeoJSON data for model rendering.
pub struct GeoJsonLoader {
    _private: (),
}

impl GeoJsonLoader {
    /// Creates a new GeoJsonLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GeoJsonLoader {
    fn default() -> Self { Self::new() }
}
