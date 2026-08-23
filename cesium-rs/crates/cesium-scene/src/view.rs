//! Ported from `packages/engine/Source/Scene/View.js`.

use cesium_core::matrix4::Matrix4;

/// Represents a view frustum for culling.
pub struct View {
    /// The view matrix.
    pub view_matrix: Matrix4,
    /// The projection matrix.
    pub projection_matrix: Matrix4,
}

impl View {
    /// Creates a new view.
    pub fn new() -> Self {
        Self {
            view_matrix: Matrix4::IDENTITY,
            projection_matrix: Matrix4::IDENTITY,
        }
    }
}

impl Default for View {
    fn default() -> Self { Self::new() }
}
