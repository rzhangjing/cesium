//! Ported from `packages/engine/Source/Scene/GlobeTranslucency.js`.
//!
//! Properties for controlling globe translucency.

/// Properties for controlling globe translucency.
///
/// When enabled, the globe is rendered semi-transparently, allowing the user
/// to see through it.
pub struct GlobeTranslucency {
    /// Whether globe translucency is enabled.
    pub enabled: bool,
    /// The alpha value for the translucent globe (0.0 = fully transparent, 1.0 = opaque).
    pub alpha: f64,
    /// Whether to show the front side of the globe when translucent.
    pub front_facing_alpha: f64,
}

impl GlobeTranslucency {
    /// Creates a new GlobeTranslucency.
    pub fn new() -> Self {
        Self {
            enabled: false,
            alpha: 1.0,
            front_facing_alpha: 1.0,
        }
    }
}

impl Default for GlobeTranslucency {
    fn default() -> Self { Self::new() }
}
