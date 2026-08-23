//! Ported from `packages/engine/Source/Scene/StencilOperation.js`.

/// A stencil test operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StencilOperation {
    /// Sets to zero.
    Zero = 0,
    /// Keeps the current value.
    Keep = 1,
    /// Replaces with the reference value.
    Replace = 2,
    /// Increments.
    Increment = 3,
    /// Decrements.
    Decrement = 4,
    /// Bitwise inverts.
    Invert = 5,
}
