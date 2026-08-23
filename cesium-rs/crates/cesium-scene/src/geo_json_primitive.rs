//! Ported from `packages/engine/Source/Scene/GeoJsonPrimitive.js`.

/// A GeoJSON primitive.
pub struct GeoJsonPrimitive {
    _private: (),
}

impl GeoJsonPrimitive {
    /// Creates a new GeoJsonPrimitive.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GeoJsonPrimitive {
    fn default() -> Self { Self::new() }
}
