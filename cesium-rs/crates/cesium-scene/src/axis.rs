//! Ported from `packages/engine/Source/Scene/Axis.js`.

/// An axis of rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Axis {
    /// The X axis.
    X = 0,
    /// The Y axis.
    Y = 1,
    /// The Z axis.
    Z = 2,
}
