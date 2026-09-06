//! Ported from `packages/engine/Source/Scene/Cesium3DTileStyleEngine.js`.

/// Engine for evaluating 3D tile styles.
///
/// Manages style expressions and their application to tile features.
pub struct Cesium3DTileStyleEngine {
    /// Whether the style has been modified since last evaluation.
    pub style_dirty: bool,
}

impl Cesium3DTileStyleEngine {
    /// Creates a new Cesium3DTileStyleEngine.
    pub fn new() -> Self { Self { style_dirty: true } }
}

impl Default for Cesium3DTileStyleEngine {
    fn default() -> Self { Self::new() }
}
