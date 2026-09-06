//! Ported from `packages/engine/Source/Scene/ConeEmitter.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::math::CesiumMath;

use crate::particle::Particle;
use crate::particle_emitter::ParticleEmitter;

const DEFAULT_ANGLE: f64 = 0.5235987755982988; // toRadians(30.0)

/// A particle emitter that emits particles within a cone.
///
/// Particles will be positioned at the tip of the cone and have initial
/// velocities going towards the base.
pub struct ConeEmitter {
    angle: f64,
}

impl ConeEmitter {
    /// Creates a new `ConeEmitter`.
    pub fn new(angle: Option<f64>) -> Self {
        Self {
            angle: angle.unwrap_or(DEFAULT_ANGLE),
        }
    }

    /// The angle of the cone in radians.
    pub fn angle(&self) -> f64 {
        self.angle
    }

    /// Sets the angle in radians.
    pub fn set_angle(&mut self, value: f64) {
        self.angle = value;
    }
}

impl ParticleEmitter for ConeEmitter {
    fn emit(&self, particle: &mut Particle) {
        let radius = self.angle.tan();

        // Compute a random point on the cone's base
        let theta = CesiumMath::random_between(0.0, CesiumMath::TWO_PI);
        let rad = CesiumMath::random_between(0.0, radius);

        let x = rad * theta.cos();
        let y = rad * theta.sin();
        let z = 1.0;

        particle.velocity = Cartesian3::normalize_new(&Cartesian3::from_elements_new(x, y, z));
        particle.position = Cartesian3::ZERO;
    }
}

impl Default for ConeEmitter {
    fn default() -> Self {
        Self::new(None)
    }
}
