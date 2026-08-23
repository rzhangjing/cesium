//! Ported from `packages/engine/Source/DataSources/BoundingSphereState.js`.

/// The state of a bounding sphere for an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BoundingSphereState {
    /// The bounding sphere is not yet computed.
    Pending = 0,
    /// The bounding sphere has been computed.
    Done = 1,
    /// The bounding sphere computation failed.
    Failed = 2,
}
