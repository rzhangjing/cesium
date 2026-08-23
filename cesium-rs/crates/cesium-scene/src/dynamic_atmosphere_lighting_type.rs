//! Ported from `packages/engine/Source/Scene/DynamicAtmosphereLightingType.js`.

/// Type of dynamic atmosphere lighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DynamicAtmosphereLightingType {
    /// Sun lighting.
    Sun = 0,
    /// Moon lighting.
    Moon = 1,
}
