//! Ported from `packages/engine/Source/Core/GeometryType.js`.
//!
//! Private enum identifying the type of geometry for internal use.

/// Private enum identifying the type of geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum GeometryType {
    /// No specific geometry type.
    None = 0,
    /// Triangle-based geometry.
    Triangles = 1,
    /// Line-based geometry.
    Lines = 2,
    /// Polyline-based geometry.
    Polylines = 3,
}
