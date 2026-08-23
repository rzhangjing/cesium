//! Ported from `packages/engine/Source/Scene/SphereEmitter.js`.

/// A particle emitter that emits from a sphere volume.
pub struct SphereEmitter {
    _private: (),
}

impl SphereEmitter {
    /// Creates a new SphereEmitter.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for SphereEmitter {
    fn default() -> Self { Self::new() }
}
