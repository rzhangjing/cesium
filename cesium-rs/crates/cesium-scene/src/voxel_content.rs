//! Ported from `packages/engine/Source/Scene/VoxelContent.js`.

/// Voxel content.
///
/// Contains the data payload of a voxel tile.
pub struct VoxelContent {
    /// Whether the content is ready.
    pub ready: bool,
    /// The number of properties.
    pub property_count: u32,
}

impl VoxelContent {
    /// Creates a new VoxelContent.
    pub fn new() -> Self { Self { ready: false, property_count: 0 } }
}

impl Default for VoxelContent {
    fn default() -> Self { Self::new() }
}
