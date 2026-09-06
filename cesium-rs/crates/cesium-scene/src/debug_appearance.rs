//! Ported from `packages/engine/Source/Scene/DebugAppearance.js`.

/// A debug appearance.
///
/// Renders geometry for debugging (normals, tangents, etc.).
pub struct DebugAppearance {
    /// The debug attribute name to visualize.
    pub attribute_name: String,
    /// Whether the appearance is visible.
    pub show: bool,
}

impl DebugAppearance {
    /// Creates a new DebugAppearance.
    pub fn new() -> Self { Self { attribute_name: "position".to_string(), show: true } }
}

impl Default for DebugAppearance {
    fn default() -> Self { Self::new() }
}
