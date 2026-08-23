//! Ported from `packages/engine/Source/Scene/BoxEmitter.js`.

/// A particle emitter that emits from a box volume.
pub struct BoxEmitter {
    _private: (),
}

impl BoxEmitter {
    /// Creates a new BoxEmitter.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BoxEmitter {
    fn default() -> Self { Self::new() }
}
