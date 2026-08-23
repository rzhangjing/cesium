//! Ported from `packages/widgets/Source/VoxelInspector/VoxelInspector.js`.

/// The VoxelInspector widget.
pub struct VoxelInspector {
    is_destroyed: bool,
}

impl VoxelInspector {
    /// Creates a new VoxelInspector.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    /// Returns whether this widget has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this widget.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for VoxelInspector {
    fn default() -> Self { Self::new() }
}
