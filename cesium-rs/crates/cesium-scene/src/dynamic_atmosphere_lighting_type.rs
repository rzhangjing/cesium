//! Ported from `packages/engine/Source/Scene/DynamicAtmosphereLightingType.js`.

/// Atmosphere lighting effects (sky atmosphere, ground atmosphere, fog) can be
/// further modified with dynamic lighting from the sun or other light source
/// that changes over time. This enum determines which light source to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DynamicAtmosphereLightingType {
    /// Do not use dynamic atmosphere lighting.
    None = 0,
    /// Use the scene's current light source for dynamic atmosphere lighting.
    SceneLight = 1,
    /// Force the dynamic atmosphere lighting to always use the sunlight direction.
    Sunlight = 2,
}

impl DynamicAtmosphereLightingType {
    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Creates from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::SceneLight),
            2 => Some(Self::Sunlight),
            _ => None,
        }
    }
}
