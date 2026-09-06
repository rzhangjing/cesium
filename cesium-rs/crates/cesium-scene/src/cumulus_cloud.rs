//! Ported from `packages/engine/Source/Scene/CumulusCloud.js`.

/// A cumulus cloud.
///
/// Represents a single cloud in a cloud collection.
pub struct CumulusCloud {
    /// Whether the cloud is visible.
    pub show: bool,
    /// The cloud position.
    pub position: (f64, f64, f64),
    /// The cloud maximum size.
    pub maximum_size: (f32, f32, f32),
}

impl CumulusCloud {
    /// Creates a new CumulusCloud.
    pub fn new() -> Self {
        Self { show: true, position: (0.0, 0.0, 0.0), maximum_size: (25.0, 25.0, 12.0) }
    }
}

impl Default for CumulusCloud {
    fn default() -> Self { Self::new() }
}
