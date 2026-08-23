//! Ported from `packages/engine/Source/Scene/BlendEquation.js`.

/// A blend equation for compositing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BlendEquation {
    /// Source + destination.
    Add = 0,
    /// Source - destination.
    Subtract = 1,
    /// Destination - source.
    ReverseSubtract = 2,
    /// Minimum of source and destination.
    Min = 3,
    /// Maximum of source and destination.
    Max = 4,
}
