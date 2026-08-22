//! Ported from `packages/engine/Source/Core/CornerType.js`.

/// Style options for corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CornerType {
    /// Corner has a smooth edge.
    Rounded = 0,
    /// Corner point is the intersection of adjacent edges.
    Mitered = 1,
    /// Corner is clipped.
    Beveled = 2,
}
