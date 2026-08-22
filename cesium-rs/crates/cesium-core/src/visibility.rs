//! Ported from `packages/engine/Source/Core/Visibility.js`.

/// This enumerated type is used in determining to what extent an object,
/// the occludee, is visible during horizon culling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Visibility {
    /// Represents that no part of an object is visible.
    None = -1,
    /// Represents that part, but not all, of an object is visible.
    Partial = 0,
    /// Represents that an object is visible in its entirety.
    Full = 1,
}
