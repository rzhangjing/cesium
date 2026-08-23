//! Ported from `packages/engine/Source/Scene/Cesium3DTilePassState.js`.

/// 3D tile pass state.
pub struct Cesium3DTilePassState {
    _private: (),
}

impl Cesium3DTilePassState {
    /// Creates a new Cesium3DTilePassState.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTilePassState {
    fn default() -> Self { Self::new() }
}
