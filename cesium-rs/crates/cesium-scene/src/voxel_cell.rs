//! Ported from `packages/engine/Source/Scene/VoxelCell.js`.

/// A single cell in a voxel volume.
///
/// Represents one voxel cell with data values.
pub struct VoxelCell {
    /// The cell index in the voxel grid.
    pub index: u32,
    /// Whether the cell is visible.
    pub show: bool,
}

impl VoxelCell {
    /// Creates a new VoxelCell.
    pub fn new() -> Self { Self { index: 0, show: true } }
}

impl Default for VoxelCell {
    fn default() -> Self { Self::new() }
}
