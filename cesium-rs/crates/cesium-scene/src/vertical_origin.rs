//! Ported from `packages/engine/Source/Scene/VerticalOrigin.js`.

/// The vertical origin of a billboard or label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum VerticalOrigin {
    /// Center.
    Center = 0,
    /// Bottom.
    Bottom = 1,
    /// Top.
    Top = -1,
}
