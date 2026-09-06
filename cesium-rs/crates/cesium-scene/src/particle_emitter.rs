//! Ported from `packages/engine/Source/Scene/ParticleEmitter.js`.

use crate::particle::Particle;

/// Base trait for particle emitters.
///
/// This type describes an interface and is not intended to be instantiated
/// directly.  Use [`BoxEmitter`](crate::box_emitter::BoxEmitter),
/// [`CircleEmitter`](crate::circle_emitter::CircleEmitter),
/// [`ConeEmitter`](crate::cone_emitter::ConeEmitter) or
/// [`SphereEmitter`](crate::sphere_emitter::SphereEmitter) instead.
pub trait ParticleEmitter {
    /// Initializes the given [`Particle`] by setting its position and velocity.
    fn emit(&self, particle: &mut Particle);
}
