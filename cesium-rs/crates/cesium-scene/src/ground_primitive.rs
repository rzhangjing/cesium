//! Ported from `packages/engine/Source/Scene/GroundPrimitive.js`.
//!
//! A ground primitive draped onto terrain.

use crate::frame_state::FrameState;
use crate::shadow_mode::ShadowMode;

/// A ground primitive that drapes geometry onto terrain or 3D Tiles.
///
/// Mirrors CesiumJS `GroundPrimitive` (1047 lines).
pub struct GroundPrimitive {
    /// Whether this primitive is shown.
    pub show: bool,
    /// Whether to allow picking.
    pub allow_picking: bool,
    /// Whether to compress geometry.
    pub compress_geometry: bool,
    /// The shadow mode.
    pub shadows: ShadowMode,
    /// Whether this primitive is ready.
    ready: bool,
    /// Whether this primitive has been destroyed.
    is_destroyed: bool,
}

impl GroundPrimitive {
    /// Creates a new GroundPrimitive.
    pub fn new() -> Self {
        Self {
            show: true,
            allow_picking: true,
            compress_geometry: true,
            shadows: ShadowMode::Disabled,
            ready: false,
            is_destroyed: false,
        }
    }

    /// Updates the primitive for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires terrain draping pipeline
    }

    /// Returns whether this primitive is ready.
    pub fn is_ready(&self) -> bool { self.ready }

    /// Returns whether this primitive has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this primitive.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for GroundPrimitive {
    fn default() -> Self { Self::new() }
}
