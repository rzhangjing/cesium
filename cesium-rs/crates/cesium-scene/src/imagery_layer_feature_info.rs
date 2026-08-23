//! Ported from `packages/engine/Source/Scene/ImageryLayerFeatureInfo.js`.

/// Feature info from an imagery layer.
pub struct ImageryLayerFeatureInfo {
    _private: (),
}

impl ImageryLayerFeatureInfo {
    /// Creates a new ImageryLayerFeatureInfo.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImageryLayerFeatureInfo {
    fn default() -> Self { Self::new() }
}
