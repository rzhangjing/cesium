//! Ported from `packages/engine/Source/Renderer/PassState.js`.
//!
//! Per-pass rendering state overrides.

use cesium_core::bounding_rectangle::BoundingRectangle;

/// The state for a particular rendering pass.
/// Used to supplement the state in a command being executed.
pub struct PassState {
    /// When defined, overrides the blending property of a DrawCommand's render state.
    pub blending_enabled: Option<bool>,
    /// When defined, overrides the scissor test property.
    pub scissor_test: Option<bool>,
    /// The viewport used when one is not defined by a DrawCommand's render state.
    pub viewport: Option<BoundingRectangle>,
}

impl PassState {
    /// Creates a new default pass state.
    pub fn new() -> Self {
        Self {
            blending_enabled: None,
            scissor_test: None,
            viewport: None,
        }
    }
}

impl Default for PassState {
    fn default() -> Self {
        Self::new()
    }
}
