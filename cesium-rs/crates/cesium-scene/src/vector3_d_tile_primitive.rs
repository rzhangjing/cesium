//! Ported from `packages/engine/Source/Scene/Vector3DTilePrimitive.js`.

/// A primitive for rendering vector 3D tiles.
///
/// Combines points, polylines, and polygons into a single renderable primitive.
pub struct Vector3DTilePrimitive {
    /// Whether the primitive is visible.
    pub show: bool,
    /// Whether the primitive is ready for rendering.
    pub ready: bool,
}

impl Vector3DTilePrimitive {
    /// Creates a new Vector3DTilePrimitive.
    pub fn new() -> Self { Self { show: true, ready: false } }
}

impl Default for Vector3DTilePrimitive {
    fn default() -> Self { Self::new() }
}
