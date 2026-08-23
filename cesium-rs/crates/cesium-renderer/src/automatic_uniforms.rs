//! Ported from `packages/engine/Source/Renderer/AutomaticUniforms.js`.
//!
//! Automatic uniforms that are set every frame by the renderer.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::matrix4::Matrix4;

/// Automatic uniforms set every frame by the renderer.
///
/// These include model-view-projection matrices, camera position,
/// time values, and other commonly-needed values.
pub struct AutomaticUniforms {
    /// The model-view-projection matrix.
    pub czm_modelViewProjection: Matrix4,
    /// The model-view matrix.
    pub czm_modelView: Matrix4,
    /// The projection matrix.
    pub czm_projection: Matrix4,
    /// The model matrix.
    pub czm_model: Matrix4,
    /// The view matrix.
    pub czm_view: Matrix4,
    /// The camera position in world coordinates.
    pub czm_viewport: [f32; 4],
    /// The camera position in eye coordinates.
    pub czm_eyeHeight: f32,
}

impl AutomaticUniforms {
    /// Creates a new set of automatic uniforms with default values.
    pub fn new() -> Self {
        Self {
            czm_modelViewProjection: Matrix4::IDENTITY,
            czm_modelView: Matrix4::IDENTITY,
            czm_projection: Matrix4::IDENTITY,
            czm_model: Matrix4::IDENTITY,
            czm_view: Matrix4::IDENTITY,
            czm_viewport: [0.0, 0.0, 1.0, 1.0],
            czm_eyeHeight: 0.0,
        }
    }
}

impl Default for AutomaticUniforms {
    fn default() -> Self { Self::new() }
}
