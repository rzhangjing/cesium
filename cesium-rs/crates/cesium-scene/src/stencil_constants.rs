//! Ported from `packages/engine/Source/Scene/StencilConstants.js`.

/// Constants for stencil buffer operations.
pub struct StencilConstants;

impl StencilConstants {
    /// The clear value for the stencil buffer.
    pub const CLEAR_VALUE: u32 = 0;
    /// The reference value for stencil testing.
    pub const REFERENCE_VALUE: u32 = 1;
    /// The write mask for stencil operations.
    pub const WRITE_MASK: u32 = 0xFF;
}
