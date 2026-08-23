//! Ported from `packages/engine/Source/Scene/GlobeTranslucencyState.js`.
//!
//! Per-frame state for globe translucency rendering.

/// Per-frame state for globe translucency rendering.
pub struct GlobeTranslucencyState {
    /// Whether the camera is underground (below the terrain surface).
    pub camera_underground: bool,
    /// The alpha value for the current frame.
    pub alpha: f64,
    /// Whether translucency is active this frame.
    pub active: bool,
}

impl GlobeTranslucencyState {
    /// Creates a new GlobeTranslucencyState.
    pub fn new() -> Self {
        Self { camera_underground: false, alpha: 1.0, active: false }
    }

    /// Updates the state for the current frame.
    pub fn update(&mut self, camera_underground: bool, alpha: f64) {
        self.camera_underground = camera_underground;
        self.alpha = alpha;
        self.active = camera_underground || alpha < 1.0;
    }
}

impl Default for GlobeTranslucencyState {
    fn default() -> Self { Self::new() }
}
