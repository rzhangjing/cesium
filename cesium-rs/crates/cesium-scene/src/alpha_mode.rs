//! Ported from `packages/engine/Source/Scene/AlphaMode.js`.

/// The alpha blending mode for a primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AlphaMode {
    /// Fully opaque.
    Opaque = 0,
    /// Alpha tested (binary transparency).
    Mask = 1,
    /// Alpha blended (translucent).
    Blend = 2,
}
