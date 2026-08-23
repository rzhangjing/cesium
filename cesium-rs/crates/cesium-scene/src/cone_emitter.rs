//! Ported from `packages/engine/Source/Scene/ConeEmitter.js`.

/// A cone-shaped particle emitter.
pub struct ConeEmitter {
    _private: (),
}

impl ConeEmitter {
    /// Creates a new ConeEmitter.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ConeEmitter {
    fn default() -> Self { Self::new() }
}
