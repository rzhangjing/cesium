//! Ported from `packages/engine/Source/Scene/DepthFunction.js`.

/// Depth test function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DepthFunction {
    /// Never pass.
    Never = 0,
    /// Less than.
    Less = 1,
    /// Equal.
    Equal = 2,
    /// Less or equal.
    LessOrEqual = 3,
    /// Greater.
    Greater = 4,
    /// Not equal.
    NotEqual = 5,
    /// Greater or equal.
    GreaterOrEqual = 6,
    /// Always pass.
    Always = 7,
}
