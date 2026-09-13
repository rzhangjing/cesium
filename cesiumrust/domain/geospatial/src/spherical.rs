//! Spherical coordinates.
//! Maps to CesiumJS `Core/Spherical.js`

use glam::DVec3;

/// A set of curvilinear 3D coordinates: clock, cone, and magnitude.
/// Maps to CesiumJS `Spherical`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spherical {
    /// The angular coordinate lying in the equatorial plane, measured from the x-axis.
    pub clock: f64,
    /// The angular coordinate measured from the z-axis (polar angle / cone angle).
    pub cone: f64,
    /// The linear coordinate measured from the origin.
    pub magnitude: f64,
}

impl Default for Spherical {
    fn default() -> Self {
        Self { clock: 0.0, cone: 0.0, magnitude: 1.0 }
    }
}

impl Spherical {
    pub fn new(clock: f64, cone: f64, magnitude: f64) -> Self {
        Self { clock, cone, magnitude }
    }

    /// Converts a Cartesian3 to Spherical coordinates.
    /// Maps to `Spherical.fromCartesian3`
    pub fn from_cartesian3(cartesian: DVec3) -> Self {
        let magnitude = cartesian.length();
        let mut cone = 0.0;
        let mut clock = 0.0;

        if magnitude > 0.0 {
            let rad = cartesian.z / magnitude;
            // Clamp to [-1, 1] for acos safety
            cone = rad.clamp(-1.0, 1.0).acos();
            clock = cartesian.y.atan2(cartesian.x);
            if clock < 0.0 {
                clock += std::f64::consts::TAU;
            }
        }

        Self { clock, cone, magnitude }
    }

    /// Returns a normalized copy (magnitude = 1.0).
    /// Maps to `Spherical.normalize`
    pub fn normalize(&self) -> Self {
        Self {
            clock: self.clock,
            cone: self.cone,
            magnitude: 1.0,
        }
    }

    /// Returns true if this spherical equals other within epsilon.
    /// Maps to `Spherical.equalsEpsilon`
    pub fn equals_epsilon(&self, other: &Self, epsilon: f64) -> bool {
        (self.clock - other.clock).abs() <= epsilon
            && (self.cone - other.cone).abs() <= epsilon
            && (self.magnitude - other.magnitude).abs() <= epsilon
    }
}

impl std::fmt::Display for Spherical {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.clock, self.cone, self.magnitude)
    }
}
