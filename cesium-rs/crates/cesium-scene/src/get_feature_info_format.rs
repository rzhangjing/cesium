//! Ported from `packages/engine/Source/Scene/getFeatureInfoFormat.js`.

/// Gets feature info format.
pub struct GetFeatureInfoFormat {
    _private: (),
}

impl GetFeatureInfoFormat {
    /// Creates a new GetFeatureInfoFormat.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GetFeatureInfoFormat {
    fn default() -> Self { Self::new() }
}
