//! Ported from `packages/engine/Source/Scene/BoxEmitter.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::math::CesiumMath;

use crate::particle::Particle;
use crate::particle_emitter::ParticleEmitter;

/// A particle emitter that emits particles within a box.
///
/// Particles will be positioned randomly within the box and have initial
/// velocities emanating from the center of the box.
pub struct BoxEmitter {
    dimensions: Cartesian3,
}

impl BoxEmitter {
    /// Creates a new `BoxEmitter`.
    ///
    /// # Panics
    /// Panics in debug builds if any dimension component is negative.
    pub fn new(dimensions: Option<Cartesian3>) -> Self {
        let dims = dimensions.unwrap_or(Cartesian3::new(1.0, 1.0, 1.0));
        debug_assert!(dims.x >= 0.0 && dims.y >= 0.0 && dims.z >= 0.0);
        Self { dimensions: dims }
    }

    /// The width, height and depth dimensions of the box in meters.
    pub fn dimensions(&self) -> &Cartesian3 {
        &self.dimensions
    }

    /// Sets the dimensions.
    pub fn set_dimensions(&mut self, value: Cartesian3) {
        debug_assert!(value.x >= 0.0 && value.y >= 0.0 && value.z >= 0.0);
        self.dimensions = value;
    }
}

impl ParticleEmitter for BoxEmitter {
    fn emit(&self, particle: &mut Particle) {
        let half = Cartesian3::multiply_by_scalar_new(&self.dimensions, 0.5);
        let x = CesiumMath::random_between(-half.x, half.x);
        let y = CesiumMath::random_between(-half.y, half.y);
        let z = CesiumMath::random_between(-half.z, half.z);

        particle.position = Cartesian3::from_elements_new(x, y, z);
        particle.velocity = Cartesian3::normalize_new(&particle.position);
    }
}

impl Default for BoxEmitter {
    fn default() -> Self {
        Self::new(None)
    }
}
