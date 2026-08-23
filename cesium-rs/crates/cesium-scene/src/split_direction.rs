//! Ported from `packages/engine/Source/Scene/SplitDirection.js`.

/// The split direction for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum SplitDirection {
    /// Render on the left side.
    Left = -1,
    /// Render on both sides.
    Both = 0,
    /// Render on the right side.
    Right = 1,
}
