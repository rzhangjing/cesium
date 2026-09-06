//! Ported from `packages/engine/Source/Scene/SphereEmitter.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::math::CesiumMath;

use crate::particle::Particle;
use crate::particle_emitter::ParticleEmitter;

/// A particle emitter that emits particles within a sphere.
///
/// Particles will be positioned randomly within the sphere and have initial
/// velocities emanating from the center of the sphere.
pub struct SphereEmitter {
    radius: f64,
}

impl SphereEmitter {
    /// Creates a new `SphereEmitter`.
    ///
    /// # Panics
    /// Panics in debug builds if `radius` is not positive.
    pub fn new(radius: Option<f64>) -> Self {
        let r = radius.unwrap_or(1.0);
        debug_assert!(r > 0.0);
        Self { radius: r }
    }

    /// The radius of the sphere in meters.
    pub fn radius(&self) -> f64 {
        self.radius
    }

    /// Sets the radius.
    pub fn set_radius(&mut self, value: f64) {
        debug_assert!(value > 0.0);
        self.radius = value;
    }
}

impl ParticleEmitter for SphereEmitter {
    fn emit(&self, particle: &mut Particle) {
        let theta = CesiumMath::random_between(0.0, CesiumMath::TWO_PI);
        let phi = CesiumMath::random_between(0.0, CesiumMath::PI);
        let rad = CesiumMath::random_between(0.0, self.radius);

        let sin_phi = phi.sin();
        let x = rad * theta.cos() * sin_phi;
        let y = rad * theta.sin() * sin_phi;
        let z = rad * phi.cos();

        particle.position = Cartesian3::from_elements_new(x, y, z);
        particle.velocity = Cartesian3::normalize_new(&particle.position);
    }
}

impl Default for SphereEmitter {
    fn default() -> Self {
        Self::new(None)
    }
}
