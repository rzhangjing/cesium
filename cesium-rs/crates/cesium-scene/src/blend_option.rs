//! Ported from `packages/engine/Source/Scene/BlendOption.js`.

/// The blending option for a primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BlendOption {
    /// Blending is disabled.
    Disabled = 0,
    /// Standard alpha blending.
    AlphaBlend = 1,
    /// Premultiplied alpha blending.
    Premultiplied = 2,
    /// Additive blending.
    Additive = 3,
}
