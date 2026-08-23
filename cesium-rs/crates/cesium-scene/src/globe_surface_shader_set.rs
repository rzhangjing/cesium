//! Ported from `packages/engine/Source/Scene/GlobeSurfaceShaderSet.js`.
//!
//! Manages the shader programs used to render globe surface tiles.

/// Manages the shader programs used to render globe surface tiles.
///
/// In CesiumJS, this generates combinations of vertex/fragment shaders based on
/// enabled features (lighting, fog, water, atmosphere, materials, etc.).
pub struct GlobeSurfaceShaderSet {
    /// Whether shaders need to be rebuilt.
    dirty: bool,
}

impl GlobeSurfaceShaderSet {
    /// Creates a new GlobeSurfaceShaderSet.
    pub fn new() -> Self {
        Self { dirty: true }
    }

    /// Marks the shader set as dirty (needing rebuild).
    pub fn make_dirty(&mut self) {
        self.dirty = true;
    }

    /// Returns whether the shader set needs rebuilding.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clears the dirty flag after shaders have been rebuilt.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }
}

impl Default for GlobeSurfaceShaderSet {
    fn default() -> Self { Self::new() }
}
