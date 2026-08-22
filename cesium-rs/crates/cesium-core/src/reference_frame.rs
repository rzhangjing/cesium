//! Ported from `packages/engine/Source/Core/ReferenceFrame.js`.

/// Constants for identifying well-known reference frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ReferenceFrame {
    /// The fixed frame.
    Fixed = 0,
    /// The inertial frame.
    Inertial = 1,
}
