//! Ported from `packages/engine/Source/Scene/Particle.js`.

/// A single particle in a particle system.
pub struct Particle {
    _private: (),
}

impl Particle {
    /// Creates a new Particle.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Particle {
    fn default() -> Self { Self::new() }
}
