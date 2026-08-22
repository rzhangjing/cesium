//! Ported from `packages/engine/Source/Core/GeometryOffsetAttribute.js`.
//!
//! Identifies which vertices should have `applyOffset = true`.

/// Which vertices should have a value of `true` for the `applyOffset` attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum GeometryOffsetAttribute {
    /// No vertices are offset.
    None = 0,
    /// Only top vertices are offset.
    Top = 1,
    /// All vertices are offset.
    All = 2,
}

impl GeometryOffsetAttribute {
    /// Returns `true` if `value` is a valid `GeometryOffsetAttribute` discriminant.
    pub fn validate(value: u32) -> bool {
        value <= 2
    }

    /// Tries to convert a raw `u32` into a `GeometryOffsetAttribute`.
    pub fn try_from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Top),
            2 => Some(Self::All),
            _ => None,
        }
    }
}
