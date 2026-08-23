//! Ported from `packages/engine/Source/DataSources/Rotation.js`.

use cesium_core::quaternion::Quaternion;

/// A rotation defined by an axis and angle.
pub struct Rotation {
    /// The rotation axis.
    pub axis: (f64, f64, f64),
    /// The rotation angle in radians.
    pub angle: f64,
}

impl Rotation {
    /// Creates a new rotation.
    pub fn new(axis: (f64, f64, f64), angle: f64) -> Self {
        Self { axis, angle }
    }

    /// Converts this rotation to a quaternion.
    pub fn to_quaternion(&self) -> Quaternion {
        let half_angle = self.angle / 2.0;
        let s = half_angle.sin();
        Quaternion::new(
            self.axis.0 * s,
            self.axis.1 * s,
            self.axis.2 * s,
            half_angle.cos(),
        )
    }
}
