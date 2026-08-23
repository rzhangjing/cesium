//! Ported from `packages/engine/Source/DataSources/StripeOrientation.js`.

/// The orientation of stripes in a stripe material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StripeOrientation {
    /// Horizontal stripes.
    Horizontal = 0,
    /// Vertical stripes.
    Vertical = 1,
}
