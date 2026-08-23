//! Ported from `packages/engine/Source/Scene/ClassificationPrimitive.js`.
//!
//! A primitive used for classifying other primitives (e.g., terrain or 3D Tiles).

use crate::frame_state::FrameState;
use crate::shadow_mode::ShadowMode;

/// A primitive used for classifying other primitives on the globe surface.
///
/// Mirrors CesiumJS `ClassificationPrimitive` (719 lines).
pub struct ClassificationPrimitive {
    /// Whether this primitive is shown.
    pub show: bool,
    /// Whether to allow picking.
    pub allow_picking: bool,
    /// Whether to compress geometry.
    pub compress_geometry: bool,
    /// The shadow mode.
    pub shadows: ShadowMode,
    /// Whether this primitive has been destroyed.
    is_destroyed: bool,
}

impl ClassificationPrimitive {
    /// Creates a new ClassificationPrimitive.
    pub fn new() -> Self {
        Self {
            show: true,
            allow_picking: true,
            compress_geometry: true,
            shadows: ShadowMode::Disabled,
            is_destroyed: false,
        }
    }

    /// Updates the primitive for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires geometry pipeline processing
    }

    /// Returns whether this primitive has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this primitive.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for ClassificationPrimitive {
    fn default() -> Self { Self::new() }
}
