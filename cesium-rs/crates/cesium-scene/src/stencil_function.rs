//! Ported from `packages/engine/Source/Scene/StencilFunction.js`.

/// A stencil test function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StencilFunction {
    /// Never passes.
    Never = 0,
    /// Passes if less.
    Less = 1,
    /// Passes if equal.
    Equal = 2,
    /// Passes if less or equal.
    LessOrEqual = 3,
    /// Passes if greater.
    Greater = 4,
    /// Passes if not equal.
    NotEqual = 5,
    /// Passes if greater or equal.
    GreaterOrEqual = 6,
    /// Always passes.
    Always = 7,
}
