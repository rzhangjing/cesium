//! Ported from `packages/engine/Source/Scene/BoundingVolumeSemantics.js`.

/// The semantics of a bounding volume in 3D Tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BoundingVolumeSemantics {
    /// The tile's bounding volume.
    BoundingVolume = 0,
    /// The content's bounding volume.
    ContentBoundingVolume = 1,
}
