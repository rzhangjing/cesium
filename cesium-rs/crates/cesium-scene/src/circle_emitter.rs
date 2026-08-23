//! Ported from `packages/engine/Source/Scene/CircleEmitter.js`.

/// A circular particle emitter.
pub struct CircleEmitter {
    _private: (),
}

impl CircleEmitter {
    /// Creates a new CircleEmitter.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CircleEmitter {
    fn default() -> Self { Self::new() }
}
