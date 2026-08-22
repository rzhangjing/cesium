//! Ported from packages/engine/Source/Core/Intersect.js
//!
//! This enumerated type is used in determining where, relative to the frustum, an
//! object is located. The object can either be fully contained within the frustum
//! (INSIDE), partially inside the frustum and partially outside (INTERSECTING), or
//! somewhere entirely outside of the frustum's 6 planes (OUTSIDE).

/// Represents where an object is located relative to the frustum.
///
/// Port of `Intersect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Intersect {
    /// Represents that an object is not contained within the frustum.
    ///
    /// Port of `Intersect.OUTSIDE`.
    Outside = -1,

    /// Represents that an object intersects one of the frustum's planes.
    ///
    /// Port of `Intersect.INTERSECTING`.
    Intersecting = 0,

    /// Represents that an object is fully within the frustum.
    ///
    /// Port of `Intersect.INSIDE`.
    Inside = 1,
}
