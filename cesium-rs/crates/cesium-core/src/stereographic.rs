//! Ported from `packages/engine/Source/Core/Stereographic.js`.
//!
//! Represents a point in stereographic coordinates.
//!
//! NOTE: Full implementation requires `EllipsoidTangentPlane`, which has not
//! yet been ported. The struct and basic accessors are provided; methods that
//! depend on `EllipsoidTangentPlane` are stubbed.

use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;

/// Represents a point in stereographic coordinates, which can be obtained by
/// projecting a cartesian coordinate from one pole onto a tangent plane at the
/// other pole.
#[derive(Clone, Debug)]
pub struct Stereographic {
    /// The stereographic position (2D coordinates).
    pub position: Cartesian2,
    // tangentPlane: EllipsoidTangentPlane — not yet ported.
    // Stored as an opaque placeholder; will be replaced once
    // EllipsoidTangentPlane is available.
    _placeholder: (),
}

impl Default for Stereographic {
    fn default() -> Self {
        Self {
            position: Cartesian2::ZERO,
            _placeholder: (),
        }
    }
}

/// Half unit sphere ellipsoid (radii = 0.5, 0.5, 0.5).
pub const HALF_UNIT_SPHERE_RADII: Cartesian3 = Cartesian3 { x: 0.5, y: 0.5, z: 0.5 };

/// North pole position on the half unit sphere.
pub const NORTH_POLE: Cartesian3 = Cartesian3 { x: 0.0, y: 0.0, z: 0.5 };

/// South pole position on the half unit sphere.
pub const SOUTH_POLE: Cartesian3 = Cartesian3 { x: 0.0, y: 0.0, z: -0.5 };

impl Stereographic {
    /// Creates a new Stereographic instance.
    pub fn new(position: Option<Cartesian2>) -> Self {
        Self {
            position: position.unwrap_or(Cartesian2::ZERO),
            _placeholder: (),
        }
    }

    /// Gets the x coordinate.
    pub fn x(&self) -> f64 {
        self.position.x
    }

    /// Gets the y coordinate.
    pub fn y(&self) -> f64 {
        self.position.y
    }

    // NOTE: The following methods require EllipsoidTangentPlane and are
    // therefore not yet implemented:
    //
    // - ellipsoid() -> &Ellipsoid
    // - conformal_latitude() -> f64
    // - longitude() -> f64
    // - get_latitude(ellipsoid) -> f64
    // - from_cartesian(cartesian, result) -> Stereographic
    // - from_cartesian_array(cartesians, result) -> Vec<Stereographic>
    // - clone_stereographic(stereographic, result) -> Option<Stereographic>
}
