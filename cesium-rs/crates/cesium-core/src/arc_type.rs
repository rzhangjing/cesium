//! Ported from `packages/engine/Source/Core/ArcType.js`.

/// ArcType defines the path that should be taken connecting vertices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ArcType {
    /// Straight line that does not conform to the surface of the ellipsoid.
    None = 0,
    /// Follow geodesic path.
    Geodesic = 1,
    /// Follow rhumb or loxodrome path.
    Rhumb = 2,
}
