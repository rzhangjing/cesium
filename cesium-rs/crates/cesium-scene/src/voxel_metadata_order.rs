//! Ported from `packages/engine/Source/Scene/VoxelMetadataOrder.js`.

/// Metadata ordering for voxel content.
///
/// In all cases, x data is contiguous in strides along the y axis,
/// and each group of y strides represents a z slice.
/// However, the orientation of the axes follows different conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VoxelMetadataOrder {
    /// The default ordering following the 3D Tiles convention. Z-axis points upward.
    ZUp = 0,
    /// The ordering following the glTF convention. Y-axis points upward.
    YUp = 1,
}

impl VoxelMetadataOrder {
    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Creates from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::ZUp),
            1 => Some(Self::YUp),
            _ => None,
        }
    }
}
