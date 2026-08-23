//! Ported from `packages/engine/Source/Scene/ParticleEmitter.js`.

/// Base class for particle emitters.
pub struct ParticleEmitter {
    _private: (),
}

impl ParticleEmitter {
    /// Creates a new ParticleEmitter.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ParticleEmitter {
    fn default() -> Self { Self::new() }
}
