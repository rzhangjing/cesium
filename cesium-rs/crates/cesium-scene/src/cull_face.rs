//! Ported from `packages/engine/Source/Scene/CullFace.js`.

/// Face culling mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CullFace {
    /// Front face.
    Front = 0,
    /// Back face.
    Back = 1,
}
