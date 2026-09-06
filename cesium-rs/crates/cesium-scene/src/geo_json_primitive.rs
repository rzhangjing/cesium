//! Ported from `packages/engine/Source/Scene/GeoJsonPrimitive.js`.

/// GeoJSON primitive.
///
/// Renders GeoJSON data as a scene primitive.
pub struct GeoJsonPrimitive {
    /// Whether the primitive is visible.
    pub show: bool,
    /// Whether the primitive is ready.
    pub ready: bool,
}

impl GeoJsonPrimitive {
    /// Creates a new GeoJsonPrimitive.
    pub fn new() -> Self { Self { show: true, ready: false } }
}

impl Default for GeoJsonPrimitive {
    fn default() -> Self { Self::new() }
}
