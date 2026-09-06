//! Ported from `packages/engine/Source/Scene/PerInstanceColorAppearance.js`.

/// Per-instance color appearance.
///
/// Renders primitives using per-instance color attributes.
pub struct PerInstanceColorAppearance {
    /// Whether the appearance is transparent.
    pub transparent: bool,
    /// Whether the appearance is flat-shaded.
    pub flat: bool,
}

impl PerInstanceColorAppearance {
    /// Creates a new PerInstanceColorAppearance.
    pub fn new() -> Self { Self { transparent: false, flat: false } }
}

impl Default for PerInstanceColorAppearance {
    fn default() -> Self { Self::new() }
}
