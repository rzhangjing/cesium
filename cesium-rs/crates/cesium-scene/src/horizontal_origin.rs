//! Ported from `packages/engine/Source/Scene/HorizontalOrigin.js`.

/// The horizontal origin of a billboard or label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum HorizontalOrigin {
    /// Center.
    Center = 0,
    /// Left.
    Left = 1,
    /// Right.
    Right = -1,
}
