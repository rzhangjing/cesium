//! Ported from `packages/engine/Source/Scene/ViewportQuad.js`.
//!
//! A viewport quad is a full-screen rectangle used for post-processing effects.

use crate::frame_state::FrameState;
use crate::material::Material;

/// A viewport quad is a full-screen rectangle used for post-processing effects.
///
/// Renders a material (shader) over the entire viewport.
pub struct ViewportQuad {
    /// Whether this quad is shown.
    pub show: bool,
    /// The material (shader) applied to the quad.
    pub material: Option<Material>,
    /// Whether this quad has been destroyed.
    is_destroyed: bool,
}

impl ViewportQuad {
    /// Creates a new ViewportQuad.
    pub fn new() -> Self {
        Self { show: true, material: None, is_destroyed: false }
    }

    /// Updates the quad for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        if !self.show { return; }
        // In full port: create a fullscreen draw command with the material's shader
    }

    /// Returns true if this object was destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys the WebGL resources held by this object.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for ViewportQuad {
    fn default() -> Self { Self::new() }
}
