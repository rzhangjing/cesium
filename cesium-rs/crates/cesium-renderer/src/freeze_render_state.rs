//! Ported from `packages/engine/Source/Renderer/freezeRenderState.js`.
//!
//! Freezes a render state object to prevent accidental modification.

use crate::render_state::RenderState;

/// Freezes a render state object to prevent accidental modification.
///
/// DEVIATION: In Rust, we use immutability (shared references) instead
/// of Object.freeze() to prevent modification.
pub fn freeze_render_state(render_state: &RenderState) -> &RenderState {
    render_state
}
