//! Ported from `packages/engine/Source/Scene/ModelAnimationLoop.js`.

/// Determines if and how a glTF animation is looped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ModelAnimationLoop {
    /// Play the animation once; do not loop it.
    None = 0,
    /// Loop the animation playing it from the start immediately after it stops.
    Repeat = 1,
    /// Loop the animation. First forward, then reverse, then forward, and so on.
    MirroredRepeat = 2,
}

impl ModelAnimationLoop {
    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Creates from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Repeat),
            2 => Some(Self::MirroredRepeat),
            _ => None,
        }
    }
}
