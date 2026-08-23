//! Ported from `packages/engine/Source/Scene/SkyBox.js`.
//!
//! A sky box drawn behind all other content.

use crate::frame_state::FrameState;

/// A sky box drawn behind all other content.
///
/// The sky box is a cube with six textures, one for each face.
pub struct SkyBox {
    pub show: bool,
    /// The positive X face image URL.
    pub sources_pos_x: Option<String>,
    /// The negative X face image URL.
    pub sources_neg_x: Option<String>,
    /// The positive Y face image URL.
    pub sources_pos_y: Option<String>,
    /// The negative Y face image URL.
    pub sources_neg_y: Option<String>,
    /// The positive Z face image URL.
    pub sources_pos_z: Option<String>,
    /// The negative Z face image URL.
    pub sources_neg_z: Option<String>,
    is_destroyed: bool,
}

impl SkyBox {
    pub fn new() -> Self {
        Self {
            show: true,
            sources_pos_x: None,
            sources_neg_x: None,
            sources_pos_y: None,
            sources_neg_y: None,
            sources_pos_z: None,
            sources_neg_z: None,
            is_destroyed: false,
        }
    }

    pub fn update(&mut self, _frame_state: &FrameState) {}

    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for SkyBox {
    fn default() -> Self { Self::new() }
}
