//! Ported from `packages/engine/Source/Scene/CircleEmitter.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::math::CesiumMath;

use crate::particle::Particle;
use crate::particle_emitter::ParticleEmitter;

/// A particle emitter that emits particles from a circle.
///
/// Particles will be positioned within a circle and have initial velocities
/// going along the z vector.
pub struct CircleEmitter {
    radius: f64,
}

impl CircleEmitter {
    /// Creates a new `CircleEmitter`.
    ///
    /// # Panics
    /// Panics in debug builds if `radius` is not positive.
    pub fn new(radius: Option<f64>) -> Self {
        let r = radius.unwrap_or(1.0);
        debug_assert!(r > 0.0);
        Self { radius: r }
    }

    /// The radius of the circle in meters.
    pub fn radius(&self) -> f64 {
        self.radius
    }

    /// Sets the radius.
    pub fn set_radius(&mut self, value: f64) {
        debug_assert!(value > 0.0);
        self.radius = value;
    }
}

impl ParticleEmitter for CircleEmitter {
    fn emit(&self, particle: &mut Particle) {
        let theta = CesiumMath::random_between(0.0, CesiumMath::TWO_PI);
        let rad = CesiumMath::random_between(0.0, self.radius);

        let x = rad * theta.cos();
        let y = rad * theta.sin();

        particle.position = Cartesian3::from_elements_new(x, y, 0.0);
        particle.velocity = Cartesian3::UNIT_Z;
    }
}

impl Default for CircleEmitter {
    fn default() -> Self {
        Self::new(None)
    }
}
