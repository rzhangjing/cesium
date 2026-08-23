//! Ported from `packages/engine/Source/Scene/HeightReference.js`.

/// Height reference mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HeightReference {
    /// None.
    None = 0,
    /// Clamp to ground.
    ClampToGround = 1,
    /// Relative to ground.
    RelativeToGround = 2,
}
