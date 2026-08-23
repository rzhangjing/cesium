//! Ported from `packages/engine/Source/Scene/ParticleBurst.js`.

/// A burst of particles emitted at once.
pub struct ParticleBurst {
    _private: (),
}

impl ParticleBurst {
    /// Creates a new ParticleBurst.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ParticleBurst {
    fn default() -> Self { Self::new() }
}
